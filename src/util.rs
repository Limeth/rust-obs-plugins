/// A workaround for not being able to use `Box<dyn Iterator<Item=I> + ExactSizeIterator>`.
/// Use `Box<dyn IteratorExactSizeIterator<I>>` instead.
pub trait IteratorExactSizeIterator<I>: Iterator<Item=I> + ExactSizeIterator {}
impl<I, J> IteratorExactSizeIterator<I> for J where J: Iterator<Item=I> + ExactSizeIterator {}
