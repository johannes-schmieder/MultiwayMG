# Mathematical contract

## Weighted three-way incidence Gramian

Let the three factors have `m1`, `m2`, and `m3` levels. Every unique tuple
`e = (a_e, b_e, c_e)` has positive weight `w_e`. Its incidence row contains
three ones, so

```text
(Bx)_e = x1[a_e] + x2[b_e] + x3[c_e].
```

The weighted Gramian is

```text
G = B' W B
```

and its energy is

```text
x' G x = sum_e w_e (x1[a_e] + x2[b_e] + x3[c_e])^2.
```

The package never needs to materialize `G` outside small dense reference and
terminal computations.

## Structural kernel

For every connected component, constants `(c1, c2, c3)` satisfying

```text
c1 + c2 + c3 = 0
```

leave all tuple sums unchanged. A convenient basis is

```text
z1 = (1, -1, 0)
z2 = (1, 0, -1).
```

`IncidenceComponents::project_structural_range` removes the Euclidean projection
onto these two vectors independently in every component. Additional null
vectors can occur under nesting or other exact incidence dependencies. The
package does not assume that connectedness implies rank deficiency exactly two;
the dense terminal uses a spectral pseudoinverse, and the preferred outer
solver is rectangular LSMR.

## Stable weighted Jacobi

Let `D = diag(G)`. For one tuple,

```text
(a + b + c)^2 <= 3(a^2 + b^2 + c^2).
```

After summing with positive weights,

```text
G <= 3D.
```

Therefore the spectrum of `D^{-1/2} G D^{-1/2}` is bounded above by three, and
weighted Jacobi with `0 < omega < 2/3` is a conservative stationary smoother on
the positive spectral subspace.

For `Q` factors, the same argument gives `G <= QD` and a stable interval
`0 < omega < 2/Q`.

## Pairwise graph structure

For factors `q` and `r`, marginalize tuple weights over the third factor. The
pair block has the form

```text
G_qr = [ D_q   C_qr ]
       [ C_qr' D_r  ].
```

Changing the sign of the second factor gives a weighted bipartite graph
Laplacian. MultiwayMG builds all three pair marginals and applies one fixed CMG
preconditioner cycle to each. Symmetric restriction and prolongation use the
partition weight `1/sqrt(2)` because every factor coordinate appears in two
pair systems.

The exact identity

```text
(a+b)^2 + (a+c)^2 + (b+c)^2
    = (a+b+c)^2 + a^2 + b^2 + c^2
```

shows that the three pair energies contain all cross-factor coupling plus one
extra diagonal copy. It motivates pair solves as a strong smoother, but does
not by itself prove a uniform condition-number bound for the additive inverse.

## Exact closure under hard factor aggregation

Let

```text
P = diag(P1, P2, P3),
```

where each fine level has exactly one parent in its own factor. Then each row of
`B P` again has exactly one active level in each factor. Hence

```text
G_c = P' G P = (B P)' W (B P).
```

Several fine tuples may map to one coarse tuple; their positive weights are
summed deterministically. Therefore the coarse matrix remains in exactly the
same weighted three-way incidence-Gramian class, and the number of unique
coarse tuples cannot exceed the fine tuple count.

## Symmetric V-cycle

The current V-cycle uses the same weighted-Jacobi correction before and after a
coarse solve. The terminal is a rank-revealing spectral pseudoinverse. With
fixed aggregation and fixed numerical weights, this defines a fixed linear
symmetric preconditioner. The pair/hierarchy hybrid uses the symmetric
composition

```text
S + (I - S G) C (I - G S) + S - S G S,
```

as realized by pair pre-smoothing, a coarse residual correction, and pair
post-smoothing. Tests check numerical symmetry directly.

## Certification

Projected PCG recomputes residuals against the submitted Gramian. Modified LSMR
works on `sqrt(W)B` and the wrapper independently recomputes

```text
||B' W (y - Bx)|| / ||B' W y||.
```

Downstream estimators should retain their own original-observation-space
certificate, normalization, and fallback logic.
