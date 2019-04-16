/// A type from which you can query the lower and upper bits
pub trait LowerUpper {
    /// One half of the bits.
    type Half;
    /// Returns `(lower, upper)`.
    fn split(&self) -> (Self::Half, Self::Half);
    /// Merges `lower` and `upper` back into `Self`.
    fn merge(lower: Self::Half, upper: Self::Half) -> Self;
}

impl LowerUpper for u32 {
    type Half = u16;
    fn split(&self) -> (Self::Half, Self::Half) {
        ((self & 0x0000_FFFF) as u16, (self >> 16) as u16)
    }
    fn merge(lower: Self::Half, upper: Self::Half) -> Self {
        u32::from(lower) | (u32::from(upper) << 16)
    }
}

#[test]
fn test_split_merge() {
    let num = 14_322_534;
    let (lower, upper) = num.split();
    let merged = LowerUpper::merge(lower, upper);
    assert_eq!(num, merged);
}
