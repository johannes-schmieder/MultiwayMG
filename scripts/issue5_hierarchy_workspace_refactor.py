#!/usr/bin/env python3
"""One-time source transformation for the issue-5 hierarchy workspace branch."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

HIERARCHY = Path("crates/multiway-mg/src/hierarchy.rs")
LIB = Path("crates/multiway-mg/src/lib.rs")
TEST = Path("crates/multiway-mg/tests/hierarchy_workspace.rs")
CHANGELOG = Path("CHANGELOG.md")


def matching_brace(source: str, opening: int) -> int:
    depth = 0
    in_string = False
    in_char = False
    escaped = False
    line_comment = False
    block_depth = 0
    index = opening
    while index < len(source):
        char = source[index]
        following = source[index + 1] if index + 1 < len(source) else ""
        if line_comment:
            if char == "\n":
                line_comment = False
            index += 1
            continue
        if block_depth:
            if char == "/" and following == "*":
                block_depth += 1
                index += 2
                continue
            if char == "*" and following == "/":
                block_depth -= 1
                index += 2
                continue
            index += 1
            continue
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            index += 1
            continue
        if in_char:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == "'":
                in_char = False
            index += 1
            continue
        if char == "/" and following == "/":
            line_comment = True
            index += 2
            continue
        if char == "/" and following == "*":
            block_depth = 1
            index += 2
            continue
        if char == '"':
            in_string = True
            index += 1
            continue
        if char == "'":
            nearby_close = source.find("'", index + 1, min(len(source), index + 5))
            if nearby_close != -1:
                in_char = True
            index += 1
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    raise RuntimeError(f"unmatched brace at byte {opening}")


@dataclass(frozen=True)
class ImplBlock:
    opening: int
    closing: int
    owner: str


@dataclass(frozen=True)
class Function:
    name: str
    start: int
    opening: int
    closing: int
    owner: str


def impl_blocks(source: str) -> list[ImplBlock]:
    blocks: list[ImplBlock] = []
    for match in re.finditer(r"\bimpl\s+([^\{]+)\{", source):
        opening = source.index("{", match.start())
        try:
            closing = matching_brace(source, opening)
        except RuntimeError:
            continue
        blocks.append(ImplBlock(opening, closing, " ".join(match.group(1).split())))
    return blocks


def functions(source: str, blocks: list[ImplBlock]) -> list[Function]:
    output: list[Function] = []
    pattern = re.compile(
        r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^\{;]*?>)?\s*\("
    )
    for block in blocks:
        region = source[block.opening + 1 : block.closing]
        cursor = 0
        while cursor < len(region):
            match = pattern.search(region, cursor)
            if not match:
                break
            start = block.opening + 1 + match.start()
            try:
                opening = source.index("{", start)
                closing = matching_brace(source, opening)
            except (ValueError, RuntimeError):
                cursor = match.end()
                continue
            if closing > block.closing:
                cursor = match.end()
                continue
            output.append(Function(match.group(1), start, opening, closing, block.owner))
            cursor = closing - (block.opening + 1) + 1
    return output


def hierarchy_root(source: str, parsed: list[Function]) -> Function:
    for function in parsed:
        if (
            function.name == "apply"
            and "Preconditioner for CycleScreenedMapHierarchy" in function.owner
        ):
            return function
    raise RuntimeError("CycleScreenedMapHierarchy Preconditioner::apply not found")


def reachable_hierarchy_functions(source: str, parsed: list[Function]) -> list[Function]:
    root = hierarchy_root(source, parsed)
    methods = {
        function.name: function
        for function in parsed
        if function.owner == "CycleScreenedMapHierarchy"
    }
    selected: dict[tuple[str, int], Function] = {(root.name, root.start): root}
    pending = [root]
    while pending:
        current = pending.pop()
        body = source[current.opening + 1 : current.closing]
        for name in re.findall(r"\bself\.([A-Za-z_][A-Za-z0-9_]*)\s*\(", body):
            candidate = methods.get(name)
            if candidate is None:
                continue
            key = (candidate.name, candidate.start)
            if key not in selected:
                selected[key] = candidate
                pending.append(candidate)
    return list(selected.values())


ZERO_DECLARATION = re.compile(
    r"let\s+mut\s+([A-Za-z_][A-Za-z0-9_]*)"
    r"(?:\s*:\s*[^=;]+)?\s*=\s*"
    r"vec!\s*\[\s*(?:0(?:\.0*)?|0_f64|0\.0_f64)\s*;"
)


def replacement_in_function(source: str, function: Function) -> list[tuple[int, int, str]]:
    body_start = function.opening + 1
    body_end = function.closing
    body = source[body_start:body_end]
    replacements: list[tuple[int, int, str]] = []
    cursor = 0
    while True:
        match = ZERO_DECLARATION.search(body, cursor)
        if match is None:
            break
        macro_relative = body.find("vec!", match.start(), match.end())
        macro = body_start + macro_relative
        bracket = source.find("[", macro, body_start + match.end())
        depth = 0
        in_string = False
        escaped = False
        index = bracket
        close = None
        while index < body_end:
            char = source[index]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
            else:
                if char == '"':
                    in_string = True
                elif char in "[({":
                    depth += 1
                elif char in "])}":
                    depth -= 1
                    if depth == 0:
                        close = index
                        break
            index += 1
        if close is None:
            raise RuntimeError(f"unterminated vec macro in {function.name}")
        semicolon = source.find(";", bracket, close)
        if semicolon < 0:
            raise RuntimeError(f"zero vec macro lacks semicolon in {function.name}")
        length = source[semicolon + 1 : close].strip()
        name = match.group(1)
        remainder = source[close + 1 : body_end]
        obvious_escape = any(
            re.search(pattern, remainder)
            for pattern in (
                rf"\breturn\s+(?:Ok\s*\(\s*)?{re.escape(name)}\b",
                rf"\bOk\s*\(\s*{re.escape(name)}\s*\)",
                rf"\bSome\s*\(\s*{re.escape(name)}\s*\)",
            )
        )
        if not obvious_escape:
            replacements.append(
                (macro, close + 1, f"hierarchy_scratch_zeroed({length})")
            )
        cursor = close + 1 - body_start
    return replacements


WORKSPACE_CODE = r'''
/// Reusable caller-owned storage for recursive hierarchy application.
///
/// The workspace is intentionally hierarchy-agnostic: it retains anonymous vector buffers only.
/// Every buffer is resized and zeroed before it is leased, so one workspace can safely be reused
/// across independently constructed hierarchies.
#[derive(Debug, Default)]
pub struct CycleScreenedMapHierarchyWorkspace {
    buffers: Vec<Vec<f64>>,
}

impl CycleScreenedMapHierarchyWorkspace {
    /// Construct an empty reusable workspace.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Heap bytes retained by scratch buffers after warm-up.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.buffers
            .iter()
            .map(|buffer| buffer.capacity() * std::mem::size_of::<f64>())
            .sum()
    }

    /// Number of retained scratch buffers.
    #[must_use]
    pub fn retained_buffer_count(&self) -> usize {
        self.buffers.len()
    }
}

#[derive(Default)]
struct ActiveHierarchyScratchPool {
    id: u64,
    buffers: Vec<Vec<f64>>,
}

std::thread_local! {
    static ACTIVE_HIERARCHY_SCRATCH: std::cell::RefCell<Vec<ActiveHierarchyScratchPool>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

static NEXT_HIERARCHY_SCRATCH_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

struct HierarchyScratchVector {
    values: Vec<f64>,
    pool_id: Option<u64>,
}

impl std::ops::Deref for HierarchyScratchVector {
    type Target = Vec<f64>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl std::ops::DerefMut for HierarchyScratchVector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl AsRef<[f64]> for HierarchyScratchVector {
    fn as_ref(&self) -> &[f64] {
        &self.values
    }
}

impl AsMut<[f64]> for HierarchyScratchVector {
    fn as_mut(&mut self) -> &mut [f64] {
        &mut self.values
    }
}

impl<'a> IntoIterator for &'a HierarchyScratchVector {
    type Item = &'a f64;
    type IntoIter = std::slice::Iter<'a, f64>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl<'a> IntoIterator for &'a mut HierarchyScratchVector {
    type Item = &'a mut f64;
    type IntoIter = std::slice::IterMut<'a, f64>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter_mut()
    }
}

impl Drop for HierarchyScratchVector {
    fn drop(&mut self) {
        let Some(pool_id) = self.pool_id else {
            return;
        };
        ACTIVE_HIERARCHY_SCRATCH.with(|stack| {
            let mut stack = stack.borrow_mut();
            if let Some(pool) = stack.last_mut().filter(|pool| pool.id == pool_id) {
                pool.buffers.push(std::mem::take(&mut self.values));
            }
        });
    }
}

fn hierarchy_scratch_zeroed(length: usize) -> HierarchyScratchVector {
    let (mut values, pool_id) = ACTIVE_HIERARCHY_SCRATCH.with(|stack| {
        let mut stack = stack.borrow_mut();
        let Some(pool) = stack.last_mut() else {
            return (Vec::new(), None);
        };
        (pool.buffers.pop().unwrap_or_default(), Some(pool.id))
    });
    values.resize(length, 0.0);
    values.fill(0.0);
    HierarchyScratchVector { values, pool_id }
}

struct HierarchyScratchScope<'a> {
    destination: &'a mut Vec<Vec<f64>>,
    id: u64,
}

impl Drop for HierarchyScratchScope<'_> {
    fn drop(&mut self) {
        let restored = ACTIVE_HIERARCHY_SCRATCH.with(|stack| {
            let mut stack = stack.borrow_mut();
            let pool = stack
                .pop()
                .expect("hierarchy scratch scope stack underflow");
            assert_eq!(pool.id, self.id, "hierarchy scratch scope order changed");
            pool.buffers
        });
        *self.destination = restored;
    }
}

fn with_hierarchy_scratch<T>(
    destination: &mut Vec<Vec<f64>>,
    operation: impl FnOnce() -> T,
) -> T {
    let id = NEXT_HIERARCHY_SCRATCH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    ACTIVE_HIERARCHY_SCRATCH.with(|stack| {
        stack.borrow_mut().push(ActiveHierarchyScratchPool {
            id,
            buffers: std::mem::take(destination),
        });
    });
    let scope = HierarchyScratchScope { destination, id };
    let result = operation();
    drop(scope);
    result
}

'''


TEST_SOURCE = r'''//! Reusable-workspace contracts for recursive hierarchy application.

#[allow(dead_code)]
#[path = "../examples/support/issue2_fixtures.rs"]
mod issue2_fixtures;
#[allow(dead_code)]
#[path = "../examples/support/issue3_recursive_fixtures.rs"]
mod issue3_recursive_fixtures;

use issue2_fixtures::{DynError, deterministic_rhs};
use issue3_recursive_fixtures::recursive_holdout_fixtures;
use multiway_mg::{CycleScreenedMapHierarchy, Preconditioner};

fn hierarchy(
    fixture: &issue3_recursive_fixtures::RecursiveHoldoutFixture,
) -> Result<CycleScreenedMapHierarchy, DynError> {
    Ok(CycleScreenedMapHierarchy::from_maps(
        fixture.problem.clone(),
        fixture.oracle_maps.clone(),
        1.0e-12,
    )?)
}

#[test]
fn hierarchy_workspace_is_exact_reusable_and_cross_instance_safe() -> Result<(), DynError> {
    let fixtures = recursive_holdout_fixtures()?;
    let first = fixtures
        .first()
        .expect("the frozen recursive fixture set is nonempty");
    let first_hierarchy = hierarchy(first)?;
    let rhs = deterministic_rhs(&first.problem)?;

    let mut reference = vec![0.0; first.problem.dimension()];
    first_hierarchy.apply(&rhs, &mut reference)?;

    let mut workspace = first_hierarchy.application_workspace();
    assert_eq!(workspace.retained_bytes(), 0);
    assert_eq!(workspace.retained_buffer_count(), 0);
    let mut reused = vec![0.0; reference.len()];
    first_hierarchy.apply_with_workspace(&rhs, &mut reused, &mut workspace)?;
    assert_eq!(reused, reference);
    let warm_bytes = workspace.retained_bytes();
    let warm_buffers = workspace.retained_buffer_count();
    assert!(warm_bytes > 0);
    assert!(warm_buffers > 0);

    for scale in [0.5, -2.0, 3.25] {
        let scaled_rhs = rhs.iter().map(|value| scale * value).collect::<Vec<_>>();
        let mut expected = vec![0.0; reference.len()];
        first_hierarchy.apply(&scaled_rhs, &mut expected)?;
        reused.fill(f64::NAN);
        first_hierarchy.apply_with_workspace(&scaled_rhs, &mut reused, &mut workspace)?;
        assert_eq!(reused, expected);
        assert_eq!(workspace.retained_bytes(), warm_bytes);
        assert_eq!(workspace.retained_buffer_count(), warm_buffers);
    }

    let independent = hierarchy(first)?;
    reused.fill(0.0);
    independent.apply_with_workspace(&rhs, &mut reused, &mut workspace)?;
    assert_eq!(reused, reference);
    assert_eq!(workspace.retained_bytes(), warm_bytes);
    assert_eq!(workspace.retained_buffer_count(), warm_buffers);

    if let Some(second) = fixtures.get(1) {
        let second_hierarchy = hierarchy(second)?;
        let second_rhs = deterministic_rhs(&second.problem)?;
        let mut expected = vec![0.0; second.problem.dimension()];
        second_hierarchy.apply(&second_rhs, &mut expected)?;
        let mut actual = vec![0.0; second.problem.dimension()];
        second_hierarchy.apply_with_workspace(&second_rhs, &mut actual, &mut workspace)?;
        assert_eq!(actual, expected);
        let second_bytes = workspace.retained_bytes();
        let second_buffers = workspace.retained_buffer_count();
        actual.fill(0.0);
        second_hierarchy.apply_with_workspace(&second_rhs, &mut actual, &mut workspace)?;
        assert_eq!(actual, expected);
        assert_eq!(workspace.retained_bytes(), second_bytes);
        assert_eq!(workspace.retained_buffer_count(), second_buffers);
    }

    let mut bad_output = vec![23.0; first.problem.dimension() - 1];
    let sentinel = bad_output.clone();
    assert!(
        first_hierarchy
            .apply_with_workspace(&rhs, &mut bad_output, &mut workspace)
            .is_err()
    );
    assert_eq!(bad_output, sentinel);
    Ok(())
}
'''


def transform() -> None:
    source = HIERARCHY.read_text()
    if "pub fn application_workspace(&self)" in source:
        print("hierarchy workspace API already present; leaving source unchanged")
        return

    struct_match = re.search(
        r"pub\s+struct\s+CycleScreenedMapHierarchy\s*\{", source
    )
    if struct_match is None:
        raise RuntimeError("CycleScreenedMapHierarchy struct not found")
    source = source[: struct_match.start()] + WORKSPACE_CODE + source[struct_match.start() :]

    blocks = impl_blocks(source)
    parsed = functions(source, blocks)
    selected = reachable_hierarchy_functions(source, parsed)
    replacements: list[tuple[int, int, str]] = []
    for function in selected:
        replacements.extend(replacement_in_function(source, function))
    if not replacements:
        fallback = [
            function
            for function in parsed
            if function.owner == "CycleScreenedMapHierarchy"
            and re.search(r"apply|cycle|level|smooth|correct", function.name)
        ]
        for function in fallback:
            replacements.extend(replacement_in_function(source, function))
    if not replacements:
        raise RuntimeError(
            "no nonescaping zero-filled vector declarations found on hierarchy application path"
        )
    for start, end, replacement in sorted(replacements, reverse=True):
        source = source[:start] + replacement + source[end:]
    print(f"replaced {len(replacements)} hierarchy scratch allocations")

    blocks = impl_blocks(source)
    parsed = functions(source, blocks)
    root = hierarchy_root(source, parsed)
    signature = source[root.start : root.opening]
    return_match = re.search(r"->\s*(Result\s*<.*>)\s*$", signature, re.S)
    if return_match is None:
        raise RuntimeError(f"unable to parse hierarchy apply return type: {signature!r}")
    return_type = " ".join(return_match.group(1).split())

    methods = f'''
    /// Construct an empty workspace for repeated hierarchy applications.
    #[must_use]
    pub fn application_workspace(&self) -> CycleScreenedMapHierarchyWorkspace {{
        CycleScreenedMapHierarchyWorkspace::new()
    }}

    /// Apply this hierarchy while retaining recursive vector scratch in `workspace`.
    ///
    /// This follows the existing `Preconditioner::apply` path exactly; only the source of
    /// zero-filled temporary vectors changes. The workspace is safe to reuse with another
    /// hierarchy because every leased buffer is resized and cleared first.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CycleScreenedMapHierarchyWorkspace,
    ) -> {return_type} {{
        with_hierarchy_scratch(&mut workspace.buffers, ||
            <Self as Preconditioner>::apply(self, rhs, output)
        )
    }}
'''
    blocks = impl_blocks(source)
    inherent = next(
        (block for block in blocks if block.owner == "CycleScreenedMapHierarchy"),
        None,
    )
    if inherent is None:
        raise RuntimeError("CycleScreenedMapHierarchy inherent impl not found")
    source = source[: inherent.closing] + methods + source[inherent.closing :]
    HIERARCHY.write_text(source)

    lib = LIB.read_text()
    if "CycleScreenedMapHierarchyWorkspace" not in lib:
        if "CycleScreenedMapHierarchy," in lib:
            lib = lib.replace(
                "CycleScreenedMapHierarchy,",
                "CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace,",
                1,
            )
        elif "CycleScreenedMapHierarchy" in lib:
            lib = lib.replace(
                "CycleScreenedMapHierarchy",
                "CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace",
                1,
            )
        else:
            raise RuntimeError("CycleScreenedMapHierarchy re-export not found")
        LIB.write_text(lib)

    TEST.parent.mkdir(parents=True, exist_ok=True)
    TEST.write_text(TEST_SOURCE)

    changelog = CHANGELOG.read_text()
    entry = (
        "- Add a reusable hierarchy application workspace that retains scoped zero-filled "
        "recursive scratch across calls while preserving the existing operation order and "
        "`Preconditioner` API.\n"
    )
    if entry not in changelog:
        lines = changelog.splitlines(keepends=True)
        insertion = 1
        for index, line in enumerate(lines):
            if line.startswith("## "):
                insertion = index + 1
                while insertion < len(lines) and not lines[insertion].strip():
                    insertion += 1
                break
        lines.insert(insertion, entry)
        CHANGELOG.write_text("".join(lines))


if __name__ == "__main__":
    transform()
