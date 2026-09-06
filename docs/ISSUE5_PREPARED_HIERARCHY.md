# Prepared supplied-map hierarchy ownership (M4a)

`PreparedHierarchyTopology` borrows one exact fine topology and consumes a vector
of factor aggregations. It owns coarse canonical keys, components, merge groups
and component bijections. Transitions link by level index, so construction needs
no self-referential owners, interior mutability, topology clones or leaked storage.
Read-only level/map/group access cannot outlive the owning hierarchy. There is
no public consuming extractor for internal transition state and no owning clone.

`HierarchyWeightFrames` borrows that structural owner and one exact immutable
fine frame. It owns every coarse numerical frame, rebuilt in order with the
existing compensated group reducer and ordinary checked frame finishing path.
No sort, component discovery, numerical-array copy or stale coarse value is
needed during replay. All coarse frames remain attached to the enclosing replay;
there is no public extractor that detaches their provenance. Simultaneously live
old and changed-weight generations remain independently valid. A value-equal
fine frame or rebuilt hierarchy is a different owner and is rejected by exact
validation.

These are building blocks for the complete M4 solver. Identity and relabeling
maps remain legal structural transitions. Structural validity alone does not
admit a map's reduction/quality, numerical MAP sweep, recursive cycle, terminal
inverse or changed-weight reuse. M4b adds frame-based projection/MAP scratch;
M4c completes the fixed serial hierarchy, drivers, certificate and repeated-RHS
interface. M4 remains open until the full vertical slice passes.

## Payload and failure contract

`PreparedHierarchyBudget` charges fine topology once, all owned arrays and
heap-allocated descriptors, plus caller-declared other live state. Structural
construction consumes the factor-map vector: its actual parent and descriptor
capacities are charged and are released on failure. Map layouts are all validated
before the first new reservation. Each transition's conservative construction
bound is checked against the already retained state before its reservations.
The reported construction peak is a conservative ledger, not measured RSS.

Numerical replay admits the complete requested numerical payload before its
first reservation. Fine topology and fine numerical arrays are counted once;
coarse topology is already part of the structural owner's exclusive payload.
All degree-correction scratch is conservatively included even though the arrays
have disjoint lifetimes. Add older coarse numerical state, old fine frames when
different, and unrelated caller buffers explicitly. Inline roots, stack,
allocator headers, new allocator excess capacity and OS memory are outside this
payload scope. Actual retained capacities are independently queryable.

No partially constructed owner is published on shape, budget, reservation or
numerical failure. Inputs borrowed by the builder are unchanged. Consumed maps
are dropped on failure; the constructor does not promise to return them. Old
published hierarchy/frame generations remain usable after a failed new build.

## Evidence

The tests compare every replayed level with fresh ordinary coarsening and the
existing single-transition replay, including bitwise weight/degree equivalence,
nonmonotone parent maps, Galerkin action, component renumbering, a terminal-only
hierarchy and multiple current generations. Extra rank deficiency remains valid;
this layer does not impose structural-rank assumptions.

A two-transition case injects both errors and unwinds at all 21 structural and
11 numerical reservation boundaries. Exact-budget and one-byte-short tests,
foreign owners and finite fine weights whose merged coarse total overflows are
covered. The permanent isolated allocation executable measures exact retained
and released structure/replay payload, including spare input descriptor capacity,
zero-allocation numerical-budget rejection, first/repeated 32 level actions,
and balanced allocation/release over 16 changed-frame cycles.

No speed or competitive-memory qualification is claimed from these small cases.
