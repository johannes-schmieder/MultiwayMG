#!/usr/bin/env python3
"""Compiler-guided one-time refactor for reusable hierarchy application scratch."""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

HIERARCHY = Path("crates/multiway-mg/src/hierarchy.rs")
LIB = Path("crates/multiway-mg/src/lib.rs")
TEST = Path("crates/multiway-mg/tests/hierarchy_workspace.rs")
CHANGELOG = Path("CHANGELOG.md")


@dataclass(frozen=True)
class Block:
    start: int
    opening: int
    closing: int
    header: str


def matching_brace(source: str, opening: int) -> int:
    depth = 0
    state = "code"
    block_depth = 0
    escaped = False
    index = opening
    while index < len(source):
        char = source[index]
        following = source[index + 1] if index + 1 < len(source) else ""
        if state == "line-comment":
            if char == "\n":
                state = "code"
        elif state == "block-comment":
            if char == "/" and following == "*":
                block_depth += 1
                index += 1
            elif char == "*" and following == "/":
                block_depth -= 1
                index += 1
                if block_depth == 0:
                    state = "code"
        elif state == "string":
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                state = "code"
        elif state == "char":
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == "'":
                state = "code"
        else:
            if char == "/" and following == "/":
                state = "line-comment"
                index += 1
            elif char == "/" and following == "*":
                state = "block-comment"
                block_depth = 1
                index += 1
            elif char == '"':
                state = "string"
            elif char == "'":
                nearby = source.find("'", index + 1, min(len(source), index + 6))
                if nearby != -1:
                    state = "char"
            elif char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    return index
        index += 1
    raise RuntimeError(f"unmatched brace at byte {opening}")


def blocks(source: str, pattern: re.Pattern[str]) -> list[Block]:
    output: list[Block] = []
    for match in pattern.finditer(source):
        opening = source.find("{", match.start(), match.end() + 2048)
        if opening == -1:
            continue
        try:
            closing = matching_brace(source, opening)
        except RuntimeError:
            continue
        output.append(Block(match.start(), opening, closing, " ".join(match.group(0).split())))
    return output


IMPL_PATTERN = re.compile(r"\bimpl(?:\s*<[^\{]*?>)?\s+[^\{]+\{")
FUNCTION_PATTERN = re.compile(
    r"(?:^|\n)\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(?:async\s+)?"
    r"fn\s+[A-Za-z_][A-Za-z0-9_]*\s*(?:<[^\{;]*?>)?\s*\(",
    re.MULTILINE,
)


def exact_impl(source: str, required: str) -> Block:
    for block in blocks(source, IMPL_PATTERN):
        header = source[block.start : block.opening]
        if required in " ".join(header.split()):
            return block
    raise RuntimeError(f"impl block not found: {required}")


def trait_apply(source: str) -> Block:
    impl = exact_impl(source, "Preconditioner for CycleScreenedMapHierarchy")
    region = source[impl.opening + 1 : impl.closing]
    match = re.search(r"\bfn\s+apply\s*\(", region)
    if match is None:
        raise RuntimeError("CycleScreenedMapHierarchy::apply not found")
    start = impl.opening + 1 + match.start()
    opening = source.find("{", start, impl.closing)
    if opening == -1:
        raise RuntimeError("CycleScreenedMapHierarchy::apply body not found")
    return Block(start, opening, matching_brace(source, opening), "fn apply")


def return_type(source: str, function: Block) -> str:
    signature = source[function.start : function.opening]
    arrow = signature.find("->")
    if arrow == -1:
        return "()"
    result = signature[arrow + 2 :].strip()
    result = re.split(r"\bwhere\b", result, maxsplit=1)[0].strip()
    if not result:
        raise RuntimeError(f"empty apply return type: {signature!r}")
    return result


WORKSPACE_CODE = r'''
/// Caller-owned reusable storage for recursive hierarchy application.
///
/// The workspace retains anonymous vector buffers only. Every leased buffer is resized and
/// zeroed before use, so one workspace can safely be reused across independently constructed
/// hierarchies and problem sizes.
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

    /// Heap bytes retained by vector scratch after warm-up.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.buffers
            .iter()
            .map(|buffer| buffer.capacity() * std::mem::size_of::<f64>())
            .sum()
    }

    /// Number of retained vector buffers.
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

#[derive(Debug, Default)]
struct HierarchyScratchVector {
    values: Vec<f64>,
    pool_id: Option<u64>,
}

impl Clone for HierarchyScratchVector {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            pool_id: None,
        }
    }
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

impl IntoIterator for HierarchyScratchVector {
    type Item = f64;
    type IntoIter = std::vec::IntoIter<f64>;

    fn into_iter(mut self) -> Self::IntoIter {
        self.pool_id = None;
        std::mem::take(&mut self.values).into_iter()
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

ZERO_DECLARATION = re.compile(
    r"let\s+mut\s+([A-Za-z_][A-Za-z0-9_]*)"
    r"(?:\s*:\s*[^=;]+)?\s*=\s*"
    r"vec!\s*\[\s*(?:0(?:\.0*)?|0_f64|0\.0_f64)\s*;"
)


def function_blocks(source: str) -> list[Block]:
    return blocks(source, FUNCTION_PATTERN)


def candidates(source: str) -> list[tuple[int, int, str, str]]:
    output: list[tuple[int, int, str, str]] = []
    for function in function_blocks(source):
        header = source[function.start : function.opening]
        if any(name in header for name in ("hierarchy_scratch_zeroed", "with_hierarchy_scratch")):
            continue
        body_start = function.opening + 1
        body_end = function.closing
        body = source[body_start:body_end]
        cursor = 0
        while True:
            match = ZERO_DECLARATION.search(body, cursor)
            if match is None:
                break
            macro = body_start + body.find("vec!", match.start(), match.end())
            bracket = source.find("[", macro, body_start + match.end())
            depth = 0
            index = bracket
            close = None
            while index < body_end:
                char = source[index]
                if char in "[({":
                    depth += 1
                elif char in "])}":
                    depth -= 1
                    if depth == 0:
                        close = index
                        break
                index += 1
            if close is None:
                raise RuntimeError(f"unterminated vec macro in {header!r}")
            semicolon = source.find(";", bracket, close)
            if semicolon == -1:
                raise RuntimeError(f"zero vec lacks semicolon in {header!r}")
            length = source[semicolon + 1 : close].strip()
            name = match.group(1)
            remainder = source[close + 1 : body_end]
            if any(
                re.search(pattern, remainder)
                for pattern in (
                    rf"\breturn\s+(?:Ok\s*\(\s*)?{re.escape(name)}\b",
                    rf"\bOk\s*\(\s*{re.escape(name)}\s*\)",
                    rf"\bSome\s*\(\s*{re.escape(name)}\s*\)",
                )
            ):
                cursor = close + 1 - body_start
                continue
            output.append((macro, close + 1, length, header.strip()))
            cursor = close + 1 - body_start
    return output


def update_export() -> None:
    text = LIB.read_text()
    if "CycleScreenedMapHierarchyWorkspace" in text:
        return
    if "CycleScreenedMapHierarchy," in text:
        text = text.replace(
            "CycleScreenedMapHierarchy,",
            "CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace,",
            1,
        )
    elif "CycleScreenedMapHierarchy" in text:
        text = text.replace(
            "CycleScreenedMapHierarchy",
            "CycleScreenedMapHierarchy, CycleScreenedMapHierarchyWorkspace",
            1,
        )
    else:
        raise RuntimeError("CycleScreenedMapHierarchy re-export not found")
    LIB.write_text(text)


def update_changelog() -> None:
    text = CHANGELOG.read_text()
    entry = (
        "- Add a reusable hierarchy application workspace that retains scoped zero-filled "
        "recursive scratch across calls while preserving the existing operation order and "
        "`Preconditioner` API.\n"
    )
    if entry in text:
        return
    lines = text.splitlines(keepends=True)
    insertion = 1
    for index, line in enumerate(lines):
        if line.startswith("## "):
            insertion = index + 1
            while insertion < len(lines) and not lines[insertion].strip():
                insertion += 1
            break
    lines.insert(insertion, entry)
    CHANGELOG.write_text("".join(lines))


def cargo_check() -> bool:
    completed = subprocess.run(
        [
            "cargo",
            "check",
            "--locked",
            "-p",
            "multiway-mg",
            "--all-features",
            "--lib",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return completed.returncode == 0


def transform() -> None:
    original = HIERARCHY.read_text()
    if "pub fn application_workspace(&self)" in original:
        if original.count("hierarchy_scratch_zeroed(") < 2:
            raise RuntimeError("workspace API exists but no application allocation uses its pool")
        TEST.parent.mkdir(parents=True, exist_ok=True)
        if not TEST.exists():
            TEST.write_text(TEST_SOURCE)
        update_export()
        update_changelog()
        return

    struct_match = re.search(
        r"pub\s+struct\s+CycleScreenedMapHierarchy\s*\{", original
    )
    if struct_match is None:
        raise RuntimeError("CycleScreenedMapHierarchy struct not found")
    source = original[: struct_match.start()] + WORKSPACE_CODE + original[struct_match.start() :]

    apply = trait_apply(source)
    result_type = return_type(source, apply)
    methods = f'''
    /// Construct an empty workspace for repeated hierarchy applications.
    #[must_use]
    pub fn application_workspace(&self) -> CycleScreenedMapHierarchyWorkspace {{
        CycleScreenedMapHierarchyWorkspace::new()
    }}

    /// Apply this hierarchy while retaining recursive vector scratch in `workspace`.
    ///
    /// This follows the existing `Preconditioner::apply` path exactly; only the source of
    /// zero-filled temporary vectors changes. Every leased buffer is resized and cleared first.
    pub fn apply_with_workspace(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CycleScreenedMapHierarchyWorkspace,
    ) -> {result_type} {{
        with_hierarchy_scratch(&mut workspace.buffers, ||
            <Self as Preconditioner>::apply(self, rhs, output)
        )
    }}
'''
    inherent = exact_impl(source, "CycleScreenedMapHierarchy")
    source = source[: inherent.closing] + methods + source[inherent.closing :]
    HIERARCHY.write_text(source)
    update_export()
    TEST.parent.mkdir(parents=True, exist_ok=True)
    TEST.write_text(TEST_SOURCE)
    update_changelog()

    if not cargo_check():
        raise RuntimeError("workspace API and scoped pool do not compile before substitutions")

    base = HIERARCHY.read_text()
    found = candidates(base)
    if not found:
        raise RuntimeError("no eligible zero-filled hierarchy vectors were found")

    # Replace one candidate at a time, retaining a substitution only when the complete library
    # remains compilable. Reverse byte order preserves earlier offsets in the current source.
    accepted = 0
    current = base
    for start, end, length, header in sorted(found, reverse=True):
        before = current
        current = current[:start] + f"hierarchy_scratch_zeroed({length})" + current[end:]
        HIERARCHY.write_text(current)
        if cargo_check():
            accepted += 1
            print(f"accepted scratch substitution in {header}")
        else:
            current = before
            HIERARCHY.write_text(current)
            print(f"rejected incompatible scratch substitution in {header}")
    if accepted == 0:
        raise RuntimeError("compiler rejected every hierarchy scratch substitution")
    if HIERARCHY.read_text().count("hierarchy_scratch_zeroed(") < 2:
        raise RuntimeError("scratch pool helper is not exercised")
    print(f"accepted {accepted} reusable hierarchy scratch substitutions")


if __name__ == "__main__":
    transform()
