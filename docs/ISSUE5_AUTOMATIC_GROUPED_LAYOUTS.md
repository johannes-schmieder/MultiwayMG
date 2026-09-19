# Explicit grouped automatic execution (M6j)

M6i's [complete-cost evidence](../benchmarks/results/2026-09-19/prepared-automatic-v1/README.md)
shows useful iterative work reduction but a full-cost slowdown. This increment
connects the already qualified grouped kernels to complete component execution.
It is an explicit application choice, with no automatic selector or performance
promotion. Structural maps, numerical criteria and original certificates retain
the M6h contract; v1 negative evidence and policies remain unchanged.

## Public boundary

`solve_prepared_automatic_batch_into` retains scalar behavior and its public
options/progress types. `solve_prepared_automatic_batch_into_with_layout` adds an
explicit `PreparedAutomaticLayout` and separate fixed-size
`PreparedAutomaticLayoutProgress`. Both delegate to the same component driver;
no numerical recurrence, constructor, screen or certificate is duplicated.
Static validation and checked caller-size arithmetic precede mutation of either
progress record or caller outputs. After admission, consume only columns whose
reports are present and accepted, as before.

| Layout | Hierarchy grouping | Hierarchy image | Large/final baseline |
| --- | --- | --- | --- |
| Scalar | none | none | original scalar |
| FineRow | fine nonterminal level | none | grouped fine rows |
| AllRow | all nonterminal levels | none | grouped fine rows |
| FineImage | fine nonterminal level | one E-vector | grouped fine rows |
| AllImage | all nonterminal levels | one shared maximum-E vector | grouped fine rows |

Dense and singleton components need no grouping. Automatic execution uses gated
LSMR; its baseline applies a weighted incidence/adjoint and fixed action, never
the baseline Gramian. Therefore even an image request allocates no baseline
tuple image. This saves 8E bytes of dead workspace. Hierarchy cycles do apply
Gramians and retain their one shared image. The public PCG-capable baseline API
is unchanged and can still explicitly allocate its Gramian image when needed.

The layout request is strict. There is no hidden recovery to a scalar layout.
A failed hierarchy group build follows ordinary typed hierarchy fallback to the
selected grouped baseline. A failed local baseline group build follows ordinary
component failure handling; failed final whole-original group setup returns its
typed error. Array admission, rejected work and final acceptance remain separate.

## Memory and lifetimes

Keep the sole canonical tuple array and current frame. Existing grouping uses
implicit canonical ranges for factor zero, one V+3 offset array and two E-length
narrow tuple-ID permutations when admissible. The hierarchy owns only an array
of selected grouping descriptors and their arrays. It borrows the structural
hierarchy; no new tuple/weight copies, nested arrays per row, inverse permutation
or RHS workspace pool is introduced.

Build groups after structural construction and before numerical replay. Admit
fine/coarse structure, original/local/caller owners, retained grouping and the
current setup cursor. Carry actual group capacity through replay and dense
factor construction. Once a solver owner exists, its payload report already
includes groups; do not charge them again as extra caller state. Screen and
solve workspaces have disjoint lifetimes, each using the existing image arena.
Coarsening never increases E, so one fine-sized image covers selected levels.

Process all K RHS before dropping the component's groups, frames, factor and
workspace. All such owners are gone before original global certification. A
final global fallback prepares its own admitted fine grouping. A rejected
hierarchy drops its full grouping before local baseline grouping is built; no
reuse of a borrowed fine group through a destroyed hierarchy is attempted.
This may repeat setup work, which must remain charged in future timing evidence.

Ordinary progress retains complete requested/admitted array peaks and typed
failures. The separate layout record retains attempt/completion/rejection counts,
published grouped-level counts, maximum actual grouping capacity, maximum logical
image length and the last failed grouping location. Completed group owners may
later be discarded after quality/solve rejection; they are not called accepted
hierarchies. Partially built unpublished groups are not called completed.
Descriptors on the stack and runtime/allocator overhead are excluded from array
bounds and belong in full-process RSS, as in the existing contract.

## Qualification

Four complete-driver scientific tests compare every coefficient bit, original
certificate, structural/screen work and native/gate work against scalar execution.
Controls cover two/three transitions, every K prefix through 32, zero columns,
changed positive weights, disconnected large/dense/singleton components, extra
null modes, rejected screens, final rank-truncation recovery, static failure,
exact budgets and denied group setup followed by successful reuse. Baseline
admission is independently reconstructed from its public owner/group/certificate
bounds, with caller live state charged exactly once.

The isolated allocator executable adds 28 grouped controls to the original 11.
Each executes two repetitions at K=1,2,4,8,16,17,32. Net allocations return to
zero, peak requested heap fits the complete admitted payload after adding
preexisting owners, and allocation records are invariant across K and repetition.
Recursive, changed-weight, forced-construction-fallback, small dense and mixed
component routes are included. Separate denied-group controls require zero
allocations. These are qualification measurements, not comparative time evidence.

Local [qualification](../benchmarks/results/2026-09-19/automatic-grouped-qualification/README.md)
passes all required Rust1.85/105 Python checks and twelve release/scientific
groups. All 39 peak records match debug/release and the original eleven match
M6h exactly; 6,240 columns certify per LSMR-enabled configuration, with four
zero-allocation group denials. Exact-source cross-platform checks and existing
scalar regression must continue to match frozen M6i numerical/work/payload
records, including the identity-control negatives. New grouped timing requires
its own committed matched policy with full costs and larger development controls.
M6 terminal/admission decisions, M7 parallelism, M8 panels, M9 changing-weight
policy and M10 competitive qualification remain open.
