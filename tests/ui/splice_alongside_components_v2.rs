use pwr_ext::view;

fn main() {
    let extra: Vec<pwr_ext::view_support::CreateComponent<'static>> = vec![];
    let _ = view! {
        components_v2 {
            text_display { "hi" }
        }
        { extra }
    };
}