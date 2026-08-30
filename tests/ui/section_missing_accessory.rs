use pwr_ext::view;
fn main() {
    let _ = view! {
        components_v2 {
            section { text_display { "hi" } }
        }
    };
}
