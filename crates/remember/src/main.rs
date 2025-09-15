fn main() {
    common::app::run("remember", |cc| {
        let ctx = libheif_rs::HeifContext::read_from_file(
            "/Users/nnord/Desktop/bottrop/Mammut/IMG_2154.HEIC",
        )
        .unwrap();
    
        let handle = ctx.primary_image_handle().unwrap();
        let mut meta_ids: Vec<libheif_rs::ItemId> = vec![0; 1];
        let count = handle.metadata_block_ids(&mut meta_ids, b"Exif");
        let exif: Vec<u8> = handle.metadata(meta_ids[0]).unwrap();

        let file =
            std::fs::File::open("/Users/nnord/Desktop/bottrop/Mammut/76c04070-6e15-4087-9629-82e9728ab702.JPG").unwrap();
        let exif = exif::Reader::new()
            .read_from_container(&mut std::io::BufReader::new(&file))
            .unwrap();
        for f in exif.fields() {
            println!("{}: {}", f.tag, f.display_value().with_unit(&exif));
        }

        return Box::new(move |ctx| {
            let ui = ctx.ui;
        });
    });
}
