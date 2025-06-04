use std::io;

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

// pub fn export<W: Write>(obj_set: &ObjSet, output: &mut W) -> Result<()> {
// }

pub fn save_file(filename: &str, write_file: impl FnOnce(&mut Write))
{

    #[cfg(target_arch = "wasm32")]
    {
        let mut buffer = Vec::new();
        let mut f = Write { inner: &mut buffer };
        write_file(&mut f);
        let base64_text = base64::encode(buffer);
        download(filename, &base64_text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let res = rfd::FileDialog::new()
            .set_file_name(filename)
            .set_title("Save File")
            .save_file();
        if let Some(path) = res {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            let mut file = Write { inner: &mut file};
            write_file(&mut file);
        }
    }
}

pub struct Write<'a>{
    pub inner: &'a  mut dyn io::Write,
}

impl<'a> io::Write for Write<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}