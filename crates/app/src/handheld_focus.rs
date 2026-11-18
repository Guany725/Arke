//! Region navigation is explicit and independent of widget construction order.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Region {
    #[default]
    Files,
    Sidebar,
    Toolbar,
}

impl Region {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Files => Self::Sidebar,
            Self::Sidebar => Self::Toolbar,
            Self::Toolbar => Self::Files,
        }
    }
}

pub(crate) fn step(index: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        return 0;
    }
    let index = index.min(count - 1);
    if forward {
        (index + 1).min(count - 1)
    } else {
        index.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regions_and_indices_stay_bounded_after_items_disappear() {
        assert_eq!(Region::Files.next().next().next(), Region::Files);
        assert_eq!(step(10, 0, true), 0);
        assert_eq!(step(10, 3, true), 2);
        assert_eq!(step(10, 3, false), 1);
        assert_eq!(step(0, 3, false), 0);
    }
}
