pub(crate) fn is_default<D: Default + Eq>(value: &D) -> bool {
    value == &D::default()
}
