/// Overlay the `Some` fields of `$new` on `$target` (the typed equivalent of the Python
/// library'"'"'s `_update`): a field missing from a server answer keeps its local value.
macro_rules! merge_some {
    ($target:expr, $new:expr; $($field:ident),+ $(,)?) => {
        $(
            if $new.$field.is_some() {
                $target.$field = $new.$field;
            }
        )+
    };
}
pub(crate) use merge_some;
