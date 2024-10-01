#![no_main]

use sp1_lib::bn254::Bn254AffinePoint;
use sp1_curves::params::FieldParameters;
sp1_zkvm::entrypoint!(main);

// generator.
// 1
// 2
const A: [u8; 64] = [
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// 2 * generator.
// 1368015179489954701390400359078579693043519447331113978918064868415326638035
// 9918110051302171585080402603319702774565515993150576347155970296011118125764
const B: [u8; 64] = [
    211, 207, 135, 109, 193, 8, 194, 211, 168, 28, 135, 22, 169, 22, 120, 217, 133, 21, 24, 104,
    91, 4, 133, 155, 2, 26, 19, 46, 231, 68, 6, 3, 196, 162, 24, 90, 122, 191, 62, 255, 199, 143,
    83, 227, 73, 164, 166, 104, 10, 156, 174, 178, 150, 95, 132, 231, 146, 124, 10, 14, 140, 115,
    237, 21,
];

// 3 * generator.
// 3353031288059533942658390886683067124040920775575537747144343083137631628272
// 19321533766552368860946552437480515441416830039777911637913418824951667761761
const C: [u8; 64] = [
    240, 171, 21, 25, 150, 85, 211, 242, 121, 230, 184, 21, 71, 216, 21, 147, 21, 189, 182, 177,
    188, 50, 2, 244, 63, 234, 107, 197, 154, 191, 105, 7, 97, 34, 254, 217, 61, 255, 241, 205, 87,
    91, 156, 11, 180, 99, 158, 49, 117, 100, 8, 141, 124, 219, 79, 85, 41, 148, 72, 224, 190, 153,
    183, 42,
];

pub fn main() {
    common_test_utils::weierstrass_add::test_weierstrass_add::<
        Bn254AffinePoint,
        { sp1_lib::bn254::N },
    >(&A, &B, &C, sp1_curves::weierstrass::bn254::Bn254BaseField::MODULUS);
}
