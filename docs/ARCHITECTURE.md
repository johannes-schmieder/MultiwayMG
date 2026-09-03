# Architecture

## Dependency direction

```text
CMG --------------------+
                        |
schwarz-precond --------+--> multiway-mg --> downstream estimators
                              ^
                              |
                       multiway-incidence
```

`multiway-incidence` owns the matrix class and has no graph-solver dependency.
`multiway-mg` owns experimental numerical methods. A downstream package such as
`fereg` should assemble regression inputs, choose routes, recover fixed-effect
coefficients, and certify residualized observations.

## Topology versus numerical state

`ThreeWayTopology` stores factor counts and canonical unique tuples.
`ThreeWayProblem` adds positive weights, square-root weights, the diagonal, and
component metadata. This separation is the beginning of a prepared-topology API
for changing-weight sequences such as PPML IRLS.

The first version rebuilds numerical hierarchies when weights change. It does
not reuse stale CMG preconditioners, dense terminals, or coarse weights.

## Solver layers

### Incidence kernels

- tuple validation and duplicate collapse;
- `B`, `B'`, `sqrt(W)B`, and `B'WB`;
- diagonal and energy;
- structural-kernel projection;
- hard factor aggregation and exact coarse tuple merging.

### Three-way hierarchy

- automatic shared-context affinity matching or caller-supplied oracle maps;
- hard factor-respecting prolongation;
- weighted-Jacobi smoothing;
- recursive V-cycle;
- rank-revealing spectral terminal;
- hierarchy complexity diagnostics.

### Pair-CMG

At each of the three factor pairs, tuple weights are marginalized into a
bipartite graph. The second factor is sign-switched, arbitrary restricted RHS
vectors are centered within pair components, and one fixed CMG cycle is applied.
The corrections are accumulated with two-sided partition-of-unity weights.

The first implementation allocates CMG workspaces per application. This is
intentionally simple and must be replaced by bounded reusable caller-owned
workspaces before production use.

### Iterative drivers

- projected PCG is a research driver for testing symmetry and spectral quality;
- modified LSMR is the preferred rank-robust driver on the rectangular weighted
  incidence operator;
- both paths expose true residual diagnostics.

## Integration boundary for fereg

A future fereg adapter should initially support exactly three categorical
intercept effects and OLS. The recommended flow is:

```text
observation tuples and weights
    -> collapse identical tuples and weighted RHS values
    -> construct or reuse topology
    -> build pair-CMG / three-way hierarchy
    -> solve outcome and regressor RHS blocks
    -> scatter fitted tuple values to observations
    -> run fereg's unchanged FE certificate
    -> accept, polish, or fall back
```

`fereg` should continue to own:

- solver routing and warnings;
- observation-order scatter/gather;
- memory admission for the full regression;
- coefficient normalization and saved FE behavior;
- finite-regressor algebra and covariance estimation;
- scientific certification and fallback.

## Deliberate first-version limitations

- three factors only at the numerical-method layer;
- intercept effects only;
- hard piecewise-constant interpolation;
- simple shared-exact-context affinity;
- dense spectral terminal;
- temporary vector allocation during V-cycles and pair applies;
- serial kernels;
- no prepared changing-weight replay;
- no block RHS implementation;
- no automatic production routing.
