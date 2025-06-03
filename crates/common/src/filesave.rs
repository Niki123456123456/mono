

#[cfg(target_arch = "wasm32")]
pub fn download(filename: &str, base64_text: &str) {
    use eframe::wasm_bindgen::JsCast as _;

    // Get the window and document
    let window = web_sys::window().expect("should have a Window");
    let document = window.document().expect("should have a Document");

    // Create anchor element
    let a: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .expect("Failed to create anchor element")
        .dyn_into()
        .expect("Failed to cast to HtmlAnchorElement");

    // Set href with base64 content
    let data_uri = format!("data:application/octet-stream;base64,{}", base64_text);
    a.set_href(&data_uri);

    // Set download attribute
    a.set_download(filename);

    // Hide the element
    a.style().set_property("display", "none").unwrap();

    // Add to document, trigger click, remove it
    document.body().unwrap().append_child(&a).unwrap();
    a.click();
    document.body().unwrap().remove_child(&a).unwrap();
}