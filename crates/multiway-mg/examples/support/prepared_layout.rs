//! Explicit uninstrumented layout protocol; the ordinary scalar TSV stays schema 2.
use multiway_incidence::{
    GroupedIndexWidth, PreparedHierarchyBudget, PreparedHierarchyGrouping,
    PreparedHierarchyTopology, PreparedTupleGrouping,
};
use multiway_mg::GroupedGramianMode;
use std::{error::Error, mem::size_of};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Scalar,
    FineRow,
    AllRow,
    FineImage,
    AllImage,
}
impl Layout {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "scalar" => Some(Self::Scalar),
            "fine-row" => Some(Self::FineRow),
            "all-row" => Some(Self::AllRow),
            "fine-image" => Some(Self::FineImage),
            "all-image" => Some(Self::AllImage),
            _ => None,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::FineRow => "fine-row",
            Self::AllRow => "all-row",
            Self::FineImage => "fine-image",
            Self::AllImage => "all-image",
        }
    }
    pub fn prefix(self, depth: usize) -> usize {
        match self {
            Self::Scalar => 0,
            Self::FineRow | Self::FineImage => depth.min(1),
            Self::AllRow | Self::AllImage => depth,
        }
    }
    pub const fn mode(self) -> GroupedGramianMode {
        match self {
            Self::FineImage | Self::AllImage => GroupedGramianMode::TupleImage,
            _ => GroupedGramianMode::RowGather,
        }
    }
}
pub struct Record {
    pub layout: Layout,
    pub nanos: u128,
    pub prefix: usize,
    pub retained: usize,
    pub setup_bound: usize,
    pub image_len: usize,
    levels: [Option<[usize; 3]>; 9],
}
impl Record {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            nanos: 0,
            prefix: 0,
            retained: 0,
            setup_bound: 0,
            image_len: 0,
            levels: [None; 9],
        }
    }
    pub fn prepare<'h, 'fine>(
        &mut self,
        hierarchy: &'h PreparedHierarchyTopology<'fine>,
        additional: usize,
        maximum: usize,
    ) -> Result<Option<PreparedHierarchyGrouping<'h, 'fine>>, Box<dyn Error>> {
        self.prefix = self.layout.prefix(hierarchy.level_count() - 1);
        for i in 0..hierarchy.level_count() {
            let topology = hierarchy.level(i).expect("bounded level inventory");
            self.levels[i] = Some([
                topology.topology().tuples().len(),
                topology.topology().level_counts().iter().sum(),
                0,
            ]);
        }
        if self.prefix == 0 {
            return Ok(None);
        }
        self.setup_bound =
            PreparedHierarchyGrouping::setup_payload_bound(hierarchy, self.prefix, additional)?;
        let grouping = PreparedHierarchyGrouping::try_new_with_budget(
            hierarchy,
            self.prefix,
            PreparedHierarchyBudget {
                maximum_payload_bytes: maximum,
                additional_live_payload_bytes: additional,
            },
        )?;
        self.setup_bound = grouping.setup_peak_payload_bound();
        self.retained = grouping.retained_payload_bytes()?;
        for i in 0..self.prefix {
            self.levels[i].as_mut().expect("selected level")[2] =
                match grouping.level(i).unwrap().index_width() {
                    GroupedIndexWidth::Narrow => 4,
                    GroupedIndexWidth::Wide => size_of::<usize>(),
                };
        }
        Ok(Some(grouping))
    }
    pub fn emit(&self, record_bytes: usize) {
        println!(
            "layout\t{}\t{}\t{}\t{}\t{}\t{}",
            self.layout.name(),
            self.prefix,
            size_of::<usize>(),
            size_of::<PreparedTupleGrouping<'_>>(),
            size_of::<Vec<f64>>(),
            record_bytes
        );
        println!("phase\tgrouping\t{}", self.nanos);
        println!(
            "layout_payload\t{}\t{}\t{}",
            self.retained, self.setup_bound, self.image_len
        );
        for (i, dimensions) in self.levels.iter().enumerate() {
            if let Some([e, v, width]) = dimensions {
                println!("layout_level\t{i}\t{e}\t{v}\t{width}");
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_layouts_have_bounded_prefixes_and_exact_names() {
        for (name, layout) in [
            ("scalar", Layout::Scalar),
            ("fine-row", Layout::FineRow),
            ("all-row", Layout::AllRow),
            ("fine-image", Layout::FineImage),
            ("all-image", Layout::AllImage),
        ] {
            assert_eq!(Layout::parse(name), Some(layout));
            assert_eq!(layout.name(), name);
            assert_eq!(layout.prefix(0), 0);
            assert!(layout.prefix(8) <= 8);
        }
        assert_eq!(Layout::FineImage.prefix(8), 1);
        assert_eq!(Layout::AllRow.prefix(8), 8);
        assert!(Layout::parse("auto").is_none());
    }
}
