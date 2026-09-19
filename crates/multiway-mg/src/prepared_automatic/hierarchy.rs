//! Lexically staged automatic structures, numerical replay and actual-tail screens.
use super::*;
use crate::{
    PreparedCycleScreenWorkspace, PreparedMapHierarchy, PreparedPairNeighborhoodCandidate,
};
use multiway_incidence::{
    FactorAggregation, HierarchyWeightFrames, PreparedHierarchyBuilder, PreparedHierarchyLimits,
    PreparedHierarchyTopology, PreparedProvisionalFrame, ProvisionalWeightInput,
};

pub(super) fn solve(
    frame: &ThreeWayWeightFrame<'_>,
    view: Option<PreparedComponentView<'_, '_>>,
    batch: &mut PreparedAutomaticBatch<'_>,
    options: PreparedAutomaticOptions,
    maximum: usize,
    other: usize,
    progress: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    let h = options.hierarchy.expect("explicit hierarchy attempt");
    progress.stage = PreparedAutomaticStage::Construction;
    let structure = structure(frame, h, maximum, other, progress)?;
    let groups = layout::hierarchy_groups(&structure, frame, maximum, other, progress)?;
    let group_bytes = groups
        .as_ref()
        .map_or(Ok(0), |g| g.retained_payload_bytes())?;
    progress.stage = PreparedAutomaticStage::NumericalReplay;
    let budget = payload_budget(maximum, add(other, group_bytes)?);
    admit(
        HierarchyWeightFrames::setup_payload_bound(&structure, frame, add(other, group_bytes)?)?,
        maximum,
        progress,
    )?;
    let frames = HierarchyWeightFrames::try_new_with_budget(&structure, frame, budget)?;
    let live = add(
        add(add(other, group_bytes)?, fine_payload(frame)?)?,
        add(
            structure.retained_payload_bytes()?,
            frames.retained_payload_bytes()?,
        )?,
    )?;
    let terminal_n = structure
        .level(structure.level_count() - 1)
        .expect("nonempty hierarchy")
        .topology()
        .total_levels();
    if terminal_n > PREPARED_DENSE_TERMINAL_LIMIT {
        return Err(stagnated(&structure));
    }
    admit(
        add(
            live,
            DensePseudoinverse::factorization_payload_bound(terminal_n)?,
        )?,
        maximum,
        progress,
    )?;
    let owner = match &groups {
        Some(groups) => PreparedMapHierarchy::try_new_with_grouping(
            &frames,
            groups,
            progress
                .layout
                .requested_layout
                .mode()
                .expect("grouped route"),
            options.terminal_relative_tolerance,
        )?,
        None => PreparedMapHierarchy::try_new(&frames, options.terminal_relative_tolerance)?,
    };
    let with_factor = add(live, owner.retained_payload_bytes()?)?;
    progress.stage = PreparedAutomaticStage::Screening;
    // Screen scratch and its application workspace die before the complete
    // Krylov workspace is allocated. Both setup costs are part of the batch.
    {
        admit(
            add(with_factor, owner.workspace_required_bytes()?)?,
            maximum,
            progress,
        )?;
        let mut cycle = owner.application_workspace()?;
        layout::record_image(&owner, progress);
        admit(
            PreparedCycleScreenWorkspace::setup_payload_report(
                &owner,
                &cycle,
                payload_budget(maximum, other),
            )?
            .total_payload_bound,
            maximum,
            progress,
        )?;
        let mut screen =
            PreparedCycleScreenWorkspace::try_new(&owner, &cycle, payload_budget(maximum, other))?;
        admit(
            add(
                owner.payload_report(&cycle, other)?.total_payload_bytes,
                screen.retained_payload_bytes()?,
            )?,
            maximum,
            progress,
        )?;
        let result = screen.screen(h.screen, h.criteria, &mut cycle);
        match result {
            Ok(result) => {
                screen_work(result.work, progress)?;
                if !result.accepted {
                    increment(&mut progress.quality_rejections, 1)?;
                    progress.last_quality_rejection = Some((
                        progress.component.expect("component hierarchy"),
                        *result.levels.last().expect("rejected tested tail"),
                    ));
                    return Err(numerical("automatic recursive cycle quality rejected"));
                }
            }
            Err(failure) => {
                screen_work(failure.work, progress)?;
                return Err(failure.source);
            }
        }
    }
    increment(&mut progress.accepted_hierarchies, 1)?;
    progress.stage = PreparedAutomaticStage::ComponentSolve;
    solve_columns(&owner, view, batch, options.lsmr, maximum, other, progress)
}

fn structure<'topology>(
    frame: &ThreeWayWeightFrame<'topology>,
    h: PreparedAutomaticHierarchyOptions,
    maximum: usize,
    other: usize,
    progress: &mut Progress<'_>,
) -> Result<PreparedHierarchyTopology<'topology>, MultiwayError> {
    let limits = PreparedHierarchyLimits {
        maximum_transitions: h.maximum_transitions,
        maximum_total_tuples: mul(frame.weights().len(), h.maximum_tuple_multiplier)?,
        maximum_total_coefficients: mul(frame.diagonal().len(), h.maximum_coefficient_multiplier)?,
        require_strict_dimension_reduction: true,
    };
    let fine_frame_bytes = frame.retained_payload_bytes()?;
    let b = payload_budget(maximum, add(other, fine_frame_bytes)?);
    admit(
        PreparedHierarchyBuilder::initial_payload_bound(frame.topology(), limits, b)?,
        maximum,
        progress,
    )?;
    let mut builder = PreparedHierarchyBuilder::try_new(frame.topology(), limits, b)?;
    let mut predecessor: Option<Vec<f64>> = None;
    while builder.current_level().topology().total_levels() > PREPARED_DENSE_TERMINAL_LIMIT {
        if builder.level_count() - 1 == h.maximum_transitions {
            return Err(MultiwayError::HierarchyStagnated {
                dimension: builder.current_level().topology().total_levels(),
                tuples: builder.current_level().topology().tuple_count(),
                limit: PREPARED_DENSE_TERMINAL_LIMIT,
            });
        }
        let map = if builder.level_count() == 1 {
            candidate(
                frame,
                h,
                payload_budget(maximum, add(other, builder.retained_payload_bytes()?)?),
                progress,
            )?
        } else {
            let owned = predecessor.is_some();
            let input = match predecessor.take() {
                Some(weights) => ProvisionalWeightInput::Owned(weights),
                None => ProvisionalWeightInput::Frame(frame),
            };
            let b = payload_budget(
                maximum,
                add(other, if owned { fine_frame_bytes } else { 0 })?,
            );
            let setup = PreparedProvisionalFrame::setup_payload_report(&builder, &input, b)?;
            admit(setup.total_payload_bound, maximum, progress)?;
            increment(&mut progress.provisional_input_tuples, setup.source_tuples)?;
            let provisional = PreparedProvisionalFrame::try_replay_last(&builder, input, b)
                .map_err(|f| MultiwayError::from(f.source))?;
            // The current topology is already part of builder's retained arrays;
            // candidate sizing charges it, so subtract that single shared owner.
            let current_topology = provisional.frame().topology().retained_payload_bytes()?;
            let all_other = add(
                other,
                add(fine_payload(frame)?, builder.retained_payload_bytes()?)?,
            )?
            .checked_sub(current_topology)
            .ok_or_else(overflow)?;
            let map = candidate(
                provisional.frame(),
                h,
                payload_budget(maximum, all_other),
                progress,
            )?;
            predecessor = Some(provisional.into_tuple_weights());
            map
        };
        increment(&mut progress.structural_attempts, 1)?;
        increment(
            &mut progress.structural_input_tuples,
            builder.current_level().topology().tuple_count(),
        )?;
        let b = payload_budget(
            maximum,
            add(
                add(other, fine_frame_bytes)?,
                predecessor
                    .as_ref()
                    .map_or(Ok(0), |w| bytes(w.capacity()))?,
            )?,
        );
        let result = builder.try_append(map, b);
        match result {
            Ok(report) => {
                if let Some(bound) = report.requested_peak_payload_bytes {
                    admit(bound, maximum, progress)?;
                }
                admit(builder.setup_peak_payload_bound(), maximum, progress)?;
            }
            Err(failure) => {
                if let Some(bound) = failure.report.requested_peak_payload_bytes {
                    progress.maximum_requested_payload_bytes =
                        progress.maximum_requested_payload_bytes.max(bound);
                }
                // Constructor's admitted high-water includes accepted temporary
                // structural work even when complexity later rejects the map.
                admit(builder.setup_peak_payload_bound(), maximum, progress)?;
                return Err(failure.source.into());
            }
        }
    }
    drop(predecessor);
    Ok(builder.finish())
}

fn candidate(
    frame: &ThreeWayWeightFrame<'_>,
    h: PreparedAutomaticHierarchyOptions,
    budget: PreparedHierarchyBudget,
    progress: &mut Progress<'_>,
) -> Result<FactorAggregation, MultiwayError> {
    let setup = match h.coverage {
        PreparedPairProposalCoverage::LegacyTopK => {
            PreparedPairNeighborhoodCandidate::setup_payload_report(frame, h.candidate, budget)?
        }
        PreparedPairProposalCoverage::AdjacentPairs => {
            PreparedPairNeighborhoodCandidate::adjacent_pairs_setup_payload_report(
                frame,
                h.candidate.minimum_affinity,
                budget,
            )?
        }
    };
    admit(
        setup.total_payload_bound,
        budget.maximum_payload_bytes,
        progress,
    )?;
    let result = match h.coverage {
        PreparedPairProposalCoverage::LegacyTopK => {
            PreparedPairNeighborhoodCandidate::try_new(frame, h.candidate, budget)
        }
        PreparedPairProposalCoverage::AdjacentPairs => {
            PreparedPairNeighborhoodCandidate::try_adjacent_pairs(
                frame,
                h.candidate.minimum_affinity,
                budget,
            )
        }
    };
    match result {
        Ok(candidate) => {
            candidate_work(candidate.work_report(), progress)?;
            Ok(candidate.into_aggregation())
        }
        Err(failure) => {
            candidate_work(failure.work, progress)?;
            Err(failure.source)
        }
    }
}
fn stagnated(h: &PreparedHierarchyTopology<'_>) -> MultiwayError {
    let t = h
        .level(h.level_count() - 1)
        .expect("nonempty hierarchy")
        .topology();
    MultiwayError::HierarchyStagnated {
        dimension: t.total_levels(),
        tuples: t.tuple_count(),
        limit: PREPARED_DENSE_TERMINAL_LIMIT,
    }
}
fn candidate_work(
    w: crate::PreparedAggregationWork,
    p: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    increment(&mut p.candidate_work.tuple_visits, w.tuple_visits)?;
    increment(&mut p.candidate_work.pair_entries, w.pair_entries)?;
    increment(&mut p.candidate_work.proposals, w.proposals)?;
    increment(&mut p.candidate_work.unique_candidates, w.unique_candidates)?;
    increment(
        &mut p.candidate_work.truncated_neighbors,
        w.truncated_neighbors,
    )?;
    increment(&mut p.candidate_work.accepted_pairs, w.accepted_pairs)
}
fn screen_work(
    w: crate::PreparedCycleScreenWork,
    p: &mut Progress<'_>,
) -> Result<(), MultiwayError> {
    increment(
        &mut p.screen_work.gramian_applications,
        w.gramian_applications,
    )?;
    increment(&mut p.screen_work.cycle_applications, w.cycle_applications)?;
    increment(&mut p.screen_work.energy_evaluations, w.energy_evaluations)?;
    increment(&mut p.screen_work.projections, w.projections)?;
    increment(&mut p.screen_work.defect_evaluations, w.defect_evaluations)
}
