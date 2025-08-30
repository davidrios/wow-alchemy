use std::{cmp, fmt, sync};

#[cfg(feature = "trimmed-debug-output")]
pub const FIRST_N_ELEMENTS: usize = 7;

pub trait HasLength {
    type Item: fmt::Debug;

    fn len2(&self) -> usize;
    fn get_first_n(&self, elements: usize) -> &[Self::Item];
}

impl<T: fmt::Debug> HasLength for &[T] {
    type Item = T;
    fn len2(&self) -> usize {
        self.len()
    }
    fn get_first_n(&self, elements: usize) -> &[Self::Item] {
        let end = cmp::min(elements, self.len());
        &self[..end]
    }
}

impl<T: fmt::Debug> HasLength for Vec<T> {
    type Item = T;
    fn len2(&self) -> usize {
        self.len()
    }
    fn get_first_n(&self, elements: usize) -> &[Self::Item] {
        let end = cmp::min(elements, self.len());
        &self[..end]
    }
}

impl<T: ?Sized + HasLength> HasLength for sync::Arc<T> {
    type Item = T::Item;
    fn len2(&self) -> usize {
        self.as_ref().len2()
    }
    fn get_first_n(&self, elements: usize) -> &[Self::Item] {
        self.as_ref().get_first_n(elements)
    }
}

#[cfg(feature = "trimmed-debug-output")]
pub fn trimmed_collection_fmt<T: HasLength + fmt::Debug>(
    n: &T,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    let first_three = n.get_first_n(FIRST_N_ELEMENTS);
    let num_elements = cmp::max(0, n.len2() - first_three.len());

    if num_elements == 0 {
        write!(f, "{:#?}", n)
    } else {
        write!(f, "{:#?} + {} elements", first_three, num_elements)
    }
}
#[cfg(not(feature = "trimmed-debug-output"))]
pub fn trimmed_collection_fmt<T: HasLength + fmt::Debug>(
    n: &T,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    write!(f, "{:#?}", n)
}

#[cfg(feature = "trimmed-debug-output")]
pub fn option_trimmed_collection_fmt<T: HasLength + fmt::Debug>(
    n: &Option<T>,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    if let Some(val) = n.as_ref() {
        let first_three = val.get_first_n(FIRST_N_ELEMENTS);
        let num_elements = cmp::max(0, val.len2() - first_three.len());

        if num_elements == 0 {
            write!(f, "Some({:#?})", n)
        } else {
            write!(f, "Some({:#?} + {} elements)", first_three, num_elements)
        }
    } else {
        write!(f, "{:#?}", n)
    }
}
#[cfg(not(feature = "trimmed-debug-output"))]
pub fn option_trimmed_collection_fmt<T: HasLength + fmt::Debug>(
    n: &Option<T>,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    write!(f, "{:#?}", n)
}
