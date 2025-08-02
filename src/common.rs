#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use crate::glfw::{Action, Key, GlfwReceiver};
use std::ffi::c_void;
use std::path::Path;
use image;
use image::GenericImage;

//local
use crate::camera::{Camera, Camera_Movement};
use crate::Shader;


//TODO: Test to see if this works :3
pub fn makeTexture(tex_path: &str, shader: &Shader) -> u32 {
    let mut texture = 0;
    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        // set the texture wrapping parameters
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32); // set texture wrapping to gl::REPEAT (default wrapping method)
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        // set texture filtering parameters
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        // load image, create texture and generate mipmaps
        let img = image::open(&Path::new(tex_path)).expect("Failed to load texture");
        let data = img.raw_pixels();
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            img.width() as i32,
            img.height() as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            &data[0] as *const u8 as *const c_void,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);

        // tell opengl for each sampler to which texture unit it belongs to (only has to be done once)
        // -------------------------------------------------------------------------------------------
        shader.useProgram(); // don't forget to activate/use the shader before setting uniforms!
        let texture1_name = c"texture1";
        shader.setInt(&texture1_name, 0);
    };
    texture
}

pub fn make_texture_array(tex_paths: &[&str], shader: &Shader) -> u32 {
    let layer_count = tex_paths.len() as i32;
    let mut tex_array = 0;
    unsafe {
        gl::GenTextures(1, &mut tex_array);
        gl::BindTexture(gl::TEXTURE_2D_ARRAY, tex_array);
        // Allocate storage: width, height and layer_count
        // assuming all images are the same size WxH:
        let img0 = image::open(Path::new(tex_paths[0])).unwrap();
        let (w, h) = (img0.width() as i32, img0.height() as i32);
        gl::TexStorage3D(
            gl::TEXTURE_2D_ARRAY,
            1,         // mip levels (or more if you want mips)
            gl::RGBA8, // internal format
            w,
            h,
            layer_count,
        );
        // Upload each layer
        for (layer, path) in tex_paths.iter().enumerate() {
            let img = image::open(Path::new(path)).expect("Failed to load layer");
            let data = img.to_rgba().into_raw();
            gl::TexSubImage3D(
                gl::TEXTURE_2D_ARRAY,
                0, // mip level
                0,
                0,
                layer as i32, // x, y, layer offset
                w,
                h,
                1, // size in x,y,1 layer
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const _,
            );
        }
        // Filtering & wrapping
        gl::TexParameteri(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);

        // Bind to our shader once
        shader.useProgram();
        shader.setInt(c"texture1", 0); // texture unit 0
    }
    tex_array
}

pub fn process_events(
    events: &GlfwReceiver<(f64, glfw::WindowEvent)>,
    firstMouse: &mut bool,
    lastX: &mut f32,
    lastY: &mut f32,
    camera: &mut Camera,
) {
    for (_, event) in glfw::flush_messages(events) {
        match event {
            glfw::WindowEvent::FramebufferSize(width, height) => {
                // make sure the viewport matches the new window dimensions; note that width and
                // height will be significantly larger than specified on retina displays.
                unsafe { gl::Viewport(0, 0, width, height) }
            }
            glfw::WindowEvent::CursorPos(xpos, ypos) => {
                let (xpos, ypos) = (xpos as f32, ypos as f32);
                if *firstMouse {
                    *lastX = xpos;
                    *lastY = ypos;
                    *firstMouse = false;
                }

                let xoffset = xpos - *lastX;
                let yoffset = *lastY - ypos; // reversed since y-coordinates go from bottom to top

                *lastX = xpos;
                *lastY = ypos;

                camera.ProcessMouseMovement(xoffset, yoffset, true);
            }
            _ => {}
        }
    }
}

pub fn processInput(window: &mut glfw::Window, deltaTime: f32, camera: &mut Camera) {
    if window.get_key(Key::Escape) == Action::Press {
        window.set_should_close(true)
    }

    if window.get_key(Key::W) == Action::Press {
        camera.ProcessKeyboard(Camera_Movement::FORWARD, deltaTime);
    }
    if window.get_key(Key::S) == Action::Press {
        camera.ProcessKeyboard(Camera_Movement::BACKWARD, deltaTime);
    }
    if window.get_key(Key::A) == Action::Press {
        camera.ProcessKeyboard(Camera_Movement::LEFT, deltaTime);
    }
    if window.get_key(Key::D) == Action::Press {
        camera.ProcessKeyboard(Camera_Movement::RIGHT, deltaTime);
    }
    if window.get_key(Key::Space) == Action::Press {
        camera.ProcessKeyboard(Camera_Movement::UP, deltaTime);
    }
    if window.get_key(Key::LeftShift) == Action::Press {
        camera.ProcessKeyboard(Camera_Movement::DOWN, deltaTime);
    }
}
