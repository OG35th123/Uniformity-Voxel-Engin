#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
extern crate glfw;

use self::glfw::Context;
extern crate gl;
use cgmath::Matrix4;
use cgmath::{Deg, Point3, Vector2, Vector3, perspective};
use std::sync::Arc;

// Local
mod shader;
use glfw::fail_on_errors;
use shader::Shader;
mod common;
use common::{process_events, processInput};
mod camera;
use camera::Camera;
mod world;
use crate::world::World;

// settings
const SCR_WIDTH: u32 = 800;
const SCR_HEIGHT: u32 = 600;

#[allow(non_snake_case)]
pub fn main() {
    let mut camera = Camera {
        Position: Point3::new(0.0, 0.0, 0.0),
        ..Camera::default()
    };

    let mut firstMouse = true;
    let mut lastX: f32 = SCR_WIDTH as f32 / 2.0;
    let mut lastY: f32 = SCR_HEIGHT as f32 / 2.0;

    // timing
    let mut deltaTime: f32; // time between current frame and last frame
    let mut lastFrame: f32 = 0.0;

    // glfw: initialize and configure
    // ------------------------------
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    // glfw window creation
    // --------------------
    let (mut window, events) = glfw
        .create_window(
            SCR_WIDTH,
            SCR_HEIGHT,
            "Voxel engine",
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create GLFW window");

    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_framebuffer_size_polling(true);

    window.set_cursor_mode(glfw::CursorMode::Disabled);

    // gl: load all OpenGL function pointers
    // ---------------------------------------
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    unsafe {
        gl::Viewport(0, 0, SCR_WIDTH as i32, SCR_HEIGHT as i32); // good hygiene
        gl::Enable(gl::DEPTH_TEST); // keep the nearest fragment
        gl::DepthFunc(gl::LESS); // default, but be explicit
        gl::CullFace(gl::BACK);
        gl::FrontFace(gl::CCW);
    }

    let chunkShader = Shader::new("src/shaders/shaderAtlas.vs", "src/shaders/shaderAtlas.fs");

    let mut world = World::new(&chunkShader);
    world.setAll();
    world.chunkRemeshAll();

    // let mut chunk = Chunk::new(&chunkShader, Vector2 { x: 0.0, y: 0.0 });

    // for x in 1..15 {
    //     for y in 1..128 {
    //         for z in 1..15 {
    //             if y >= 125 {
    //                 chunk.set(Vector3 { x: x, y: y, z: z }, BlockId::Grass);
    //             } else {
    //                 chunk.set(Vector3 { x: x, y: y, z: z }, BlockId::Dirt);
    //             }
    //         }
    //     }
    // }
    // chunk.set(Vector3 { x: 1, y: 1, z: 1 }, BlockId::Dirt);
    // chunk.set(Vector3 { x: 1, y: 2, z: 1 }, BlockId::Dirt);
    //
    // chunk.remesh();

    // render loop
    // -----------
    while !window.should_close() {
        // events
        // -----
        let currentFrame = glfw.get_time() as f32;
        deltaTime = currentFrame - lastFrame;
        lastFrame = currentFrame;

        // events
        // -----
        process_events(
            &events,
            &mut firstMouse,
            &mut lastX,
            &mut lastY,
            &mut camera,
        );

        // input
        // -----
        processInput(&mut window, deltaTime, &mut camera);

        // render
        // ------
        unsafe {
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            let projection: Matrix4<f32> = perspective(
                Deg(45.0),
                SCR_WIDTH as f32 / SCR_HEIGHT as f32,
                0.1,
                16.0 * 16.0,
            );
            let view = camera.GetViewMatrix();

            // let pos = camera.Position;

            // println!("world position: {:?}, local position: {:?}", pos, World::worldToLoc(pos));

            // println!("{:?}", pos);

            // let front = camera.Front;
            //
            // println!("{:?}", front);

            world.renderAll(&projection, &view);
            // chunk.draw(&projection, &view);
        }

        // glfw: swap buffers and poll IO events (keys pressed/released, mouse moved etc.)
        // -------------------------------------------------------------------------------
        window.swap_buffers();
        glfw.poll_events();
    }
}
