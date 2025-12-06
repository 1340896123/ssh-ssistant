use base64::{engine::general_purpose, Engine as _};
use image::ImageOutputFormat;
use std::io::Cursor;
use tauri::command;

#[cfg(target_os = "windows")]
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*, Win32::Storage::FileSystem::*,
    Win32::UI::Shell::*, Win32::UI::WindowsAndMessaging::*,
};

#[command]
pub fn get_file_icon(extension: String) -> std::result::Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            let mut shfi: SHFILEINFOW = std::mem::zeroed();
            let flags = SHGFI_ICON | SHGFI_USEFILEATTRIBUTES | SHGFI_SMALLICON;

            // Normalize extension
            let ext = if extension.starts_with('.') {
                extension.clone()
            } else {
                format!(".{}", extension)
            };

            let wide_ext: Vec<u16> = ext.encode_utf16().chain(std::iter::once(0)).collect();

            // Explicitly call SHGetFileInfoW
            let result = SHGetFileInfoW(
                PCWSTR(wide_ext.as_ptr()),
                FILE_ATTRIBUTE_NORMAL,
                Some(&mut shfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags,
            );

            if result == 0 {
                return Err("Failed to get file info".to_string());
            }

            if shfi.hIcon.is_invalid() {
                return Err("Invalid icon handle".to_string());
            }

            // Create a bitmap from the icon
            let hdc = GetDC(None);
            let hdc_mem = CreateCompatibleDC(Some(hdc));
            let h_bitmap = CreateCompatibleBitmap(hdc, 16, 16); // Small icon is 16x16
            let old_obj = SelectObject(hdc_mem, h_bitmap.into());

            // Draw the icon into the bitmap
            DrawIconEx(
                hdc_mem,
                0,
                0,
                shfi.hIcon,
                16,
                16,
                0,
                Some(HBRUSH::default()),
                DI_NORMAL,
            );

            SelectObject(hdc_mem, old_obj);

            let mut bitmap: BITMAP = std::mem::zeroed();
            GetObjectW(
                h_bitmap.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bitmap as *mut _ as *mut _),
            );

            let mut bi = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bitmap.bmWidth,
                biHeight: -bitmap.bmHeight, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };

            let width = bitmap.bmWidth;
            let height = bitmap.bmHeight.abs();
            let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];

            GetDIBits(
                hdc,
                h_bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bi as *mut _ as *mut _,
                DIB_RGB_COLORS,
            );

            // Cleanup GDI objects
            let _ = DeleteObject(h_bitmap.into());
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc);
            DestroyIcon(shfi.hIcon);

            // Convert BGRA to RGBA
            for i in (0..pixels.len()).step_by(4) {
                // Swap Blue and Red
                let b = pixels[i];
                pixels[i] = pixels[i + 2];
                pixels[i + 2] = b;
            }

            // Create image buffer
            let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
                width as u32,
                height as u32,
                pixels,
            )
            .ok_or_else(|| "Failed to create image buffer".to_string())?;

            let mut bytes: Vec<u8> = Vec::new();
            img.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Png)
                .map_err(|e| e.to_string())?;

            let base64_str = general_purpose::STANDARD.encode(&bytes);
            Ok(format!("data:image/png;base64,{}", base64_str))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Not supported on this OS".to_string())
    }
}
