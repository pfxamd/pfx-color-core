use pfx_color_core::{extract_image_palette, ImagePaletteOptions, ImageRegion, ImageError};
fn near(a:f64,b:f64){assert!((a-b).abs()<1e-10, "{a} != {b}");}
#[test]
fn known_three_color_population_and_proportions(){
    let mut rgba=Vec::new();
    for (pixel,n) in [([255_u8,0,0,255],8),([0,255,0,255],4),([0,0,255,255],4)] {
        for _ in 0..n {rgba.extend_from_slice(&pixel);}
    }
    let r=extract_image_palette(&rgba,4,4,ImagePaletteOptions{count:3,..Default::default()}).unwrap();
    assert_eq!(r.eligible_pixels,16);
    assert_eq!(r.colors.len(),3);
    assert_eq!(r.colors[0].population,8);
    near(r.colors[0].proportion,0.5);
    near(r.colors.iter().map(|c|c.proportion).sum(),1.0);
}
#[test]
fn transparent_and_white_pixels_filtered(){
    let rgba=[255_u8,0,0,255,255,255,255,255,0,255,0,100,0,0,255,0];
    let r=extract_image_palette(&rgba,4,1,ImagePaletteOptions{
        ignore_near_white:true,..Default::default()
    }).unwrap();
    assert_eq!(r.sampled_pixels,4);
    assert_eq!(r.eligible_pixels,1);
    assert_eq!(r.colors.len(),1);
    assert_eq!(r.colors[0].color.channels(),[1.0,0.0,0.0]);
}
#[test]
fn region_and_deterministic_bounded_sampling(){
    let mut rgba=Vec::new();
    for y in 0..32{for x in 0..32{rgba.extend_from_slice(&[(x*8)as u8,(y*8)as u8,128,255]);}}
    let opts=ImagePaletteOptions{count:7,max_samples:20,region:Some(ImageRegion{
        x:16,y:0,width:16,height:32
    }),..Default::default()};
    let first=extract_image_palette(&rgba,32,32,opts).unwrap();
    let second=extract_image_palette(&rgba,32,32,opts).unwrap();
    assert_eq!(first,second);
    assert!(first.sampled_pixels<=20);
    near(first.colors.iter().map(|c|c.proportion).sum(),1.0);
}
#[test]
fn monochrome_only_one_cluster_and_error_inputs(){
    let rgba=[120_u8,120,120,255].repeat(16);
    let r=extract_image_palette(&rgba,4,4,ImagePaletteOptions::default()).unwrap();
    assert_eq!(r.colors.len(),1);
    assert_eq!(r.colors[0].population,16);
    assert_eq!(extract_image_palette(&rgba,0,4,ImagePaletteOptions::default()),Err(ImageError::InvalidBuffer));
    assert_eq!(extract_image_palette(&rgba,usize::MAX,4,ImagePaletteOptions::default()),Err(ImageError::InvalidDimensions));
    assert_eq!(extract_image_palette(&rgba,4,4,ImagePaletteOptions{count:0,..Default::default()}),Err(ImageError::InvalidCount));
    assert_eq!(extract_image_palette(&rgba,4,4,ImagePaletteOptions{stride:0,..Default::default()}),Err(ImageError::InvalidSampling));
    assert_eq!(extract_image_palette(&rgba,4,4,ImagePaletteOptions{
        region:Some(ImageRegion{x:3,y:0,width:2,height:1}),..Default::default()
    }),Err(ImageError::InvalidRegion));
    assert_eq!(extract_image_palette(&[0,0,0,0],1,1,ImagePaletteOptions::default()),Err(ImageError::EmptyImage));
}
