use crate::air::Block;
use crate::chips::alu_base::BaseAluValueCols;
use crate::chips::alu_ext::ExtAluValueCols;
use crate::chips::batch_fri::BatchFRICols;
use crate::BaseAluIo;
use crate::BatchFRIEvent;
use crate::ExtAluIo;
use p3_baby_bear::BabyBear;

#[link(name = "sp1_recursion_core_sys", kind = "static")]
extern "C-unwind" {
    pub fn alu_base_event_to_row_babybear(
        io: &BaseAluIo<BabyBear>,
        cols: &mut BaseAluValueCols<BabyBear>,
    );
    pub fn alu_ext_event_to_row_babybear(
        io: &ExtAluIo<Block<BabyBear>>,
        cols: &mut ExtAluValueCols<BabyBear>,
    );
    pub fn batch_fri_event_to_row_babybear(
        io: &BatchFRIEvent<BabyBear>,
        cols: &mut BatchFRICols<BabyBear>,
    );
}
