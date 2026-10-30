//! Icône d'un programme Windows, au format PNG.

use std::path::Path;

/// Extrait la plus grande icône (256 px) d'un `.exe` et la renvoie en PNG.
#[cfg(windows)]
pub fn extract_png(exe: &Path) -> Option<Vec<u8>> {
    use std::mem::{size_of, zeroed};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits,
        GetObjectW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO, PrivateExtractIconsW};

    let wide: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut icon = null_mut();
    let mut icon_id = 0u32;
    // SAFETY : appels Win32 sur des pointeurs valides ; chaque ressource GDI créée est libérée avant le retour.
    unsafe {
        if PrivateExtractIconsW(wide.as_ptr(), 0, 256, 256, &mut icon, &mut icon_id, 1, 0) == 0 || icon.is_null() {
            return None;
        }
        let mut info: ICONINFO = zeroed();
        if GetIconInfo(icon, &mut info) == 0 {
            DestroyIcon(icon);
            return None;
        }
        let png = (|| {
            let mut bitmap: BITMAP = zeroed();
            if GetObjectW(info.hbmColor, size_of::<BITMAP>() as i32, &mut bitmap as *mut BITMAP as *mut _) == 0 {
                return None;
            }
            let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
            if width <= 0 || height <= 0 {
                return None;
            }
            let mut header: BITMAPINFO = zeroed();
            header.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // lignes de haut en bas
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                ..zeroed()
            };
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            let dc = CreateCompatibleDC(null_mut());
            let rows = GetDIBits(
                dc,
                info.hbmColor,
                0,
                height as u32,
                pixels.as_mut_ptr() as *mut _,
                &mut header,
                DIB_RGB_COLORS,
            );
            DeleteDC(dc);
            if rows == 0 {
                return None;
            }
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2); // BGRA -> RGBA
            }
            // Une icône sans canal de transparence sort avec alpha = 0 partout : on la rend opaque.
            if pixels.chunks_exact(4).all(|p| p[3] == 0) {
                pixels.chunks_exact_mut(4).for_each(|p| p[3] = 255);
            }
            let mut out = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().ok()?.write_image_data(&pixels).ok()?;
            }
            Some(out)
        })();
        DeleteObject(info.hbmColor);
        DeleteObject(info.hbmMask);
        DestroyIcon(icon);
        png
    }
}

#[cfg(not(windows))]
pub fn extract_png(_exe: &Path) -> Option<Vec<u8>> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn icone_d_un_programme_windows() {
        let png = extract_png(Path::new(r"C:\Windows\explorer.exe")).expect("explorer.exe a une icône");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(png.len() > 500);
        assert!(extract_png(Path::new(r"C:\nope\absent.exe")).is_none());
    }
}
