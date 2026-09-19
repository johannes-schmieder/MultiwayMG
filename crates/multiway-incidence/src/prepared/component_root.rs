//! Component-local owning roots, built only when local sparse execution needs them.
use super::*;
use crate::{
    ComponentWeightRange, PreparedComponentLayout, PreparedComponentRecoding,
    PreparedComponentView, PreparedHierarchyBudget, ThreeWayWeightFrame, WeightFrameInputKind,
};

/// A local canonical root tied to one exact component of the original layout.
///
/// Keys move directly into this owner. Original component discovery proves the
/// root is connected, so no second union/find, root-label scratch or copied input
/// keys are needed. The temporary inverse is not borrowed by the returned root.
/// Connected whole-problem callers should borrow their original topology/frame
/// instead of requesting this explicitly owning local representation.
///
/// Local roots cannot outlive their original component layout:
/// ```compile_fail
/// use multiway_incidence::*;
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let root;
/// {
///     let layout = PreparedComponentLayout::try_new(&t, PreparedHierarchyBudget::UNLIMITED).unwrap();
///     let recoding = PreparedComponentRecoding::try_new(&layout, PreparedHierarchyBudget::UNLIMITED).unwrap();
///     root = PreparedComponentRoot::try_new(&recoding, 0, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// }
/// println!("{:?}", root.topology());
/// ```
/// Local numerical frames cannot outlive their local topology:
/// ```compile_fail
/// use multiway_incidence::*;
/// let t = PreparedThreeWayTopology::try_from_collapsed([1;3], &[[0;3]]).unwrap();
/// let source = ThreeWayWeightFrame::try_new(&t, WeightFrameInput::UnitTuples).unwrap();
/// let layout = PreparedComponentLayout::try_new(&t, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// let recoding = PreparedComponentRecoding::try_new(&layout, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// let frame;
/// {
///     let root = PreparedComponentRoot::try_new(&recoding, 0, PreparedHierarchyBudget::UNLIMITED).unwrap();
///     frame = root.try_weight_frame(&source, PreparedHierarchyBudget::UNLIMITED).unwrap();
/// }
/// println!("{:?}", frame.weights());
/// ```
#[derive(Debug)]
pub struct PreparedComponentRoot<'layout, 'topology> {
    layout: &'layout PreparedComponentLayout<'topology>,
    component: usize,
    topology: PreparedThreeWayTopology,
    setup_peak_payload_bound: usize,
}

impl<'layout, 'topology> PreparedComponentRoot<'layout, 'topology> {
    /// Construct three local arrays after source/layout/inverse/live admission.
    pub fn try_new(
        recoding: &PreparedComponentRecoding<'layout, 'topology>,
        component: usize,
        budget: PreparedHierarchyBudget,
    ) -> Result<Self, IncidenceError> {
        Self::build_with(recoding, component, budget, &mut |_| Ok(()))
    }
    /// Complete requested topology-construction payload, including the live inverse.
    pub fn setup_payload_bound(
        recoding: &PreparedComponentRecoding<'_, '_>,
        component: usize,
        budget: PreparedHierarchyBudget,
    ) -> Result<usize, IncidenceError> {
        let view = component_view(recoding.layout(), component)?;
        sum_bytes(&[
            recoding.layout().topology().retained_payload_bytes()?,
            recoding.layout().retained_payload_bytes()?,
            recoding.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
            array_bytes::<[u32; 3]>(view.tuple_count())?,
            array_bytes::<usize>(view.dimension())?,
            array_bytes::<[usize; 3]>(1)?,
        ])
    }
    fn build_with<F>(
        recoding: &PreparedComponentRecoding<'layout, 'topology>,
        component: usize,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<Self, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = Self::setup_payload_bound(recoding, component, budget)?;
        admit(required, budget)?;
        let view = component_view(recoding.layout(), component)?;
        let mut tuples = reserve(view.tuple_count(), "local component tuples", before)?;
        recoding.for_each_key(component, |_, key| tuples.push(key))?;
        let mut labels = reserve(view.dimension(), "local component labels", before)?;
        labels.resize(view.dimension(), 0usize);
        let mut factor_sizes = reserve(1, "local component factor sizes", before)?;
        factor_sizes.push(view.level_counts());
        let topology = PreparedThreeWayTopology {
            topology: ThreeWayTopology::new(view.level_counts(), tuples)?,
            partition: Partition {
                labels,
                factor_sizes,
            },
            groups: None,
        };
        let actual = sum_bytes(&[
            recoding.layout().topology().retained_payload_bytes()?,
            recoding.layout().retained_payload_bytes()?,
            recoding.retained_payload_bytes()?,
            topology.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
        ])?;
        admit(actual, budget)?;
        Ok(Self {
            layout: recoding.layout(),
            component,
            topology,
            setup_peak_payload_bound: required.max(actual),
        })
    }
    /// Exact original component, for target gathering and coefficient scattering.
    pub fn component(&self) -> PreparedComponentView<'layout, 'topology> {
        self.layout
            .component(self.component)
            .expect("validated component")
    }
    /// Owned local topology; it has one supported exact component.
    pub const fn topology(&self) -> &PreparedThreeWayTopology {
        &self.topology
    }
    /// Actual exclusive local-key/label/factor-size array capacity.
    pub fn retained_payload_bytes(&self) -> Result<usize, IncidenceError> {
        self.topology.retained_payload_bytes()
    }
    /// Largest admitted requested or actual-capacity construction bound.
    pub const fn setup_peak_payload_bound(&self) -> usize {
        self.setup_peak_payload_bound
    }
    /// Complete current-generation frame construction bound.
    ///
    /// Original topology, layout, local root and submitted original frame are
    /// charged once. Add the inverse if it is still live, old frames and caller
    /// arrays explicitly. The gathered weights move into the returned frame.
    pub fn weight_frame_setup_payload_bound(
        &self,
        source: &ThreeWayWeightFrame<'_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<usize, IncidenceError> {
        source.validate_for(self.layout.topology())?;
        let view = self.component();
        sum_bytes(&[
            self.existing_frame_payload(source, budget)?,
            array_bytes::<f64>(view.tuple_count())?,
            array_bytes::<f64>(view.tuple_count())?,
            array_bytes::<f64>(view.dimension())?,
            array_bytes::<f64>(view.dimension())?,
            array_bytes::<ComponentWeightRange>(1)?,
        ])
    }
    /// Gather one original component and finish all numerical quantities afresh.
    /// No temporary gathered-weight copy survives alongside the returned weights.
    pub fn try_weight_frame(
        &self,
        source: &ThreeWayWeightFrame<'_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<ThreeWayWeightFrame<'_>, IncidenceError> {
        self.frame_with(source, budget, &mut |_| Ok(()))
    }
    fn frame_with<F>(
        &self,
        source: &ThreeWayWeightFrame<'_>,
        budget: PreparedHierarchyBudget,
        before: &mut F,
    ) -> Result<ThreeWayWeightFrame<'_>, IncidenceError>
    where
        F: FnMut(&'static str) -> Result<(), IncidenceError>,
    {
        let required = self.weight_frame_setup_payload_bound(source, budget)?;
        admit(required, budget)?;
        let view = self.component();
        let mut weights = reserve(view.tuple_count(), "local component weights", before)?;
        view.tuple_ids()
            .for_each(|id| weights.push(source.weights()[id]));
        let frame = ThreeWayWeightFrame::finish_with(
            &self.topology,
            weights,
            WeightFrameInputKind::Tuples,
            view.tuple_count(),
            before,
        )?;
        admit(
            sum_bytes(&[
                self.existing_frame_payload(source, budget)?,
                frame.retained_payload_bytes()?,
            ])?,
            budget,
        )?;
        Ok(frame)
    }
    fn existing_frame_payload(
        &self,
        source: &ThreeWayWeightFrame<'_>,
        budget: PreparedHierarchyBudget,
    ) -> Result<usize, IncidenceError> {
        sum_bytes(&[
            self.layout.topology().retained_payload_bytes()?,
            self.layout.retained_payload_bytes()?,
            self.topology.retained_payload_bytes()?,
            source.retained_payload_bytes()?,
            budget.additional_live_payload_bytes,
        ])
    }
}

fn component_view<'layout, 'topology>(
    layout: &'layout PreparedComponentLayout<'topology>,
    component: usize,
) -> Result<PreparedComponentView<'layout, 'topology>, IncidenceError> {
    layout
        .component(component)
        .ok_or(IncidenceError::ComponentIndexOutOfBounds {
            component,
            count: layout.component_count(),
        })
}
fn admit(required: usize, budget: PreparedHierarchyBudget) -> Result<(), IncidenceError> {
    if required > budget.maximum_payload_bytes {
        Err(IncidenceError::HierarchyBudgetExceeded {
            required,
            budget: budget.maximum_payload_bytes,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "component_root_tests.rs"]
mod tests;
