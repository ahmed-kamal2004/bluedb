use super::super::pool::page::Page;
pub struct MngmntPageView<'a> {
    pub page: &'a mut Page,
}

impl<'a> MngmntPageView<'a> {
    pub fn new(page: &'a mut Page) -> Self {
        MngmntPageView { page }
    }
}
