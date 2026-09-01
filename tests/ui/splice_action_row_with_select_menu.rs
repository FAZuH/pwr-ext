use pwr_ext::view;

fn main() {
    let _ = view! {
        action_row {
            { vec![pwr_ext::view_support::CreateButton::new("b").label("B")] }
            select_menu { custom_id: "s" }
        }
    };
}