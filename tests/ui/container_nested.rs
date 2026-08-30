use pwr_ext::view;
fn main() {
    let _ = view! {
        components_v2 {
            container {
                container { text_display { "nested" } }
            }
        }
    };
}
