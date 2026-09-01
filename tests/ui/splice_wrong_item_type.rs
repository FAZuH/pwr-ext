use pwr_ext::view;

fn main() {
    let wrong: Vec<&str> = vec!["a", "b"];
    let _ = view! {
        action_row {
            { wrong }
        }
    };
}