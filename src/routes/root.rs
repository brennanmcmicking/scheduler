use crate::components;
use hypertext::{html_elements, rsx, Renderable, Rendered};

pub fn root() -> Rendered<String> {
    return components::base(rsx! {
        <p>"Hello from p tag!"</p>
        {components::button::c(rsx! {"Hello"})}
    })
    .render();
}
