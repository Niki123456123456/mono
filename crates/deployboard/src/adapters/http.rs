#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_blocking(request: &ehttp::Request) -> Result<ehttp::Response, String> {
    let response = ehttp::fetch_blocking(request)?;

    return Ok(response);
}

#[cfg(target_arch = "wasm32")]
pub fn fetch_blocking(request: &ehttp::Request) -> Result<ehttp::Response, String> {
    return Err("not implemented".to_string());
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch(request: &ehttp::Request, include : bool) -> Result<ehttp::Response, String> {
    let response = ehttp::fetch_blocking(request)?;

    return Ok(response);
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch(request: &ehttp::Request, include : bool) -> Result<ehttp::Response, String> {
    use wasm_bindgen_futures::wasm_bindgen::JsCast;

    let opts = web_sys::RequestInit::new();

    opts.set_method(&request.method);
    //opts.set_mode(web_sys::RequestMode::Cors);
    if include {
        opts.set_credentials(web_sys::RequestCredentials::Include); 
    }
    

    let r = web_sys::Request::new_with_str_and_init(&request.url, &opts)
        .map_err(|x| format!("{:?}", x))?;
    for (name, value) in request.headers.headers.iter() {
        let _ = r.headers().set(name, value);
    }
    let window = web_sys::window().unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&r))
        .await
        .map_err(|x| format!("{:?}", x))?;
    let resp: web_sys::Response = resp_value.dyn_into().unwrap();

    let buffer_promise = resp.array_buffer().map_err(|x| format!("{:?}", x))?;
    let buffer = wasm_bindgen_futures::JsFuture::from(buffer_promise).await.map_err(|x| format!("{:?}", x))?;

    // Convert ArrayBuffer to Uint8Array and then to Vec<u8>
    let uint8_array = web_sys::js_sys::Uint8Array::new(&buffer);
    let mut body = vec![0; uint8_array.length() as usize];
    uint8_array.copy_to(&mut body);

    let result = ehttp::Response {
        url: request.url.clone(),
        ok: true,
        status: resp.status(),
        status_text: resp.status_text(),
        headers: ehttp::Headers::new(&[]),
        bytes: body,
    };

    return Ok(result);
}
