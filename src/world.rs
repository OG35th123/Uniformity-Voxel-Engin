use cgmath::{Matrix4, Vector2};
use cgmath::{SquareMatrix, Vector3};
use crossbeam::{channel, thread};
use gl::types::*;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::Mutex;
use std::{mem, ptr};

//local
use crate::Shader;
use crate::common::make_texture_array;

//settings
const CHUNKSIZE: usize = 16;
const CHUNKHIEGHT: usize = 128;
const RENDERDISTANCE: usize = 128;
const THREADS: usize = 12;
const vertices: [f32; 120] = [
    // back  (‑Z)
    -0.5, -0.5, -0.5, 0.0, 0.0, // 0
    0.5, -0.5, -0.5, 1.0, 0.0, // 1
    0.5, 0.5, -0.5, 1.0, 1.0, // 2
    -0.5, 0.5, -0.5, 0.0, 1.0, // 3
    // front (+Z)
    -0.5, -0.5, 0.5, 0.0, 0.0, // 4
    0.5, -0.5, 0.5, 1.0, 0.0, // 5
    0.5, 0.5, 0.5, 1.0, 1.0, // 6
    -0.5, 0.5, 0.5, 0.0, 1.0, // 7
    // left  (‑X)
    -0.5, -0.5, -0.5, 0.0, 0.0, // 8
    -0.5, -0.5, 0.5, 1.0, 0.0, // 9
    -0.5, 0.5, 0.5, 1.0, 1.0, //10
    -0.5, 0.5, -0.5, 0.0, 1.0, //11
    // right (+X)
    0.5, -0.5, -0.5, 0.0, 0.0, //12
    0.5, -0.5, 0.5, 1.0, 0.0, //13
    0.5, 0.5, 0.5, 1.0, 1.0, //14
    0.5, 0.5, -0.5, 0.0, 1.0, //15
    // bottom (‑Y)
    -0.5, -0.5, -0.5, 0.0, 1.0, //16
    0.5, -0.5, -0.5, 1.0, 1.0, //17
    0.5, -0.5, 0.5, 1.0, 0.0, //18
    -0.5, -0.5, 0.5, 0.0, 0.0, //19
    // top   (+Y)
    -0.5, 0.5, -0.5, 0.0, 1.0, //20
    0.5, 0.5, -0.5, 1.0, 1.0, //21
    0.5, 0.5, 0.5, 1.0, 0.0, //22
    -0.5, 0.5, 0.5, 0.0, 0.0, //23
];

const DIRS: [[i16; 3]; 6] = [
    [0, 0, -1], //back
    [0, 0, 1],  //front
    [-1, 0, 0], //left
    [1, 0, 0],  //right
    [0, -1, 0], //down
    [0, 1, 0],  //up
];

struct Job {
    cx: usize,
    cz: usize,
}

pub struct World<'a> {
    chunks: [[Chunk<'a>; RENDERDISTANCE]; RENDERDISTANCE],
}

impl<'a> World<'a> {
    pub fn new(shader: &'a Shader) -> Self {
        let texture = make_texture_array(
            &["src/textures/txDirt.png", "src/textures/txGrass.png"],
            shader,
        );
        let chunks: [[Chunk; RENDERDISTANCE]; RENDERDISTANCE] = std::array::from_fn(|x| {
            std::array::from_fn(|z| {
                Chunk::new(
                    shader,
                    Vector2 {
                        x: x as i32,
                        y: z as i32,
                    },
                    texture,
                )
            })
        });
        Self { chunks }
    }

    fn fillChunk(chunk: &mut Chunk) {
        for x in 0..16 {
            for y in 0..CHUNKHIEGHT {
                for z in 0..16 {
                    if x == 15 {
                        chunk.set(Vector3 { x, y, z }, BlockId::Grass);
                    } else {
                        chunk.set(Vector3 { x, y, z }, BlockId::Dirt);
                    }
                    if z == 8 && y >= 122 {
                        chunk.set(Vector3 { x, y, z }, BlockId::Air);
                    }
                    // if y >= 120 {
                    //     let r = rand::random_range(0..3);
                    //     if r == 0 {
                    //         chunk.set(Vector3 { x, y, z }, BlockId::Grass);
                    //     } else {
                    //         chunk.set(Vector3 { x, y, z }, BlockId::Air);
                    //     }
                    // } else {
                    //     chunk.set(Vector3 { x, y, z }, BlockId::Dirt);
                    // }
                }
            }
        }
    }

    pub fn setAll(&mut self) {
        let (tx, rx) = channel::unbounded::<Job>();

        for cx in 0..RENDERDISTANCE {
            for cz in 0..RENDERDISTANCE {
                tx.send(Job { cx, cz }).unwrap();
            }
        }
        drop(tx);

        let chunks = Arc::new(Mutex::new(&mut self.chunks));

        thread::scope(|s| {
            for _ in 0..THREADS {
                let rx = rx.clone();
                let chunks_clone = Arc::clone(&chunks);
                s.spawn(move |_| {
                    while let Ok(job) = rx.recv() {
                        let mut chunks = chunks_clone.lock().unwrap();
                        let chunk = &mut chunks[job.cx][job.cz];
                        World::fillChunk(chunk);
                    }
                });
            }
        })
        .unwrap();
    }

    pub fn chunkRemeshAll(&mut self) {
        let world_ptr: *const World = self; // raw const pointer → no borrow clash
        for row in self.chunks.iter_mut() {
            for chunk in row {
                unsafe {
                    chunk.remesh(&*world_ptr);
                }
            }
        }
    }

    pub fn renderAll(&self, proj: &Matrix4<f32>, view: &Matrix4<f32>) {
        for i in 0..self.chunks.len() {
            for chunk in &self.chunks[i] {
                chunk.draw(proj, view);
            }
        }
    }
    pub fn getBlockType(&self, pos: Vector2<i32>, blockPos: Vector3<usize>) -> BlockId {
        let x = pos.x;
        let z = pos.y;
        let mut blockPosX = blockPos.x;
        let mut blockPosZ = blockPos.z;

        if x < 0 || x >= RENDERDISTANCE as i32 || z < 0 || z >= RENDERDISTANCE as i32 {
            return BlockId::Air; // outside loaded world → treat as air
        }

        if blockPosX == CHUNKSIZE {
            blockPosX = CHUNKSIZE - 1;
        }
        if blockPosZ == CHUNKSIZE {
            blockPosZ = CHUNKSIZE - 1;
        }
        self.chunks[x as usize][z as usize].blocks[blockPosX][blockPos.y][blockPosZ]
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BlockId {
    Air = 3,
    Dirt = 0,
    Grass = 1,
}

pub struct Chunk<'a> {
    blocks: Box<[[[BlockId; CHUNKSIZE]; CHUNKHIEGHT]; CHUNKSIZE]>,
    shader: &'a Shader,
    VAO: u32,
    VBO: u32,
    EBO: u32,
    texture: u32,
    verts: Vec<f32>,
    vertexCount: i32,
    indexCount: i32,
    pos: Vector2<i32>,
}

impl<'a> Chunk<'a> {
    pub fn new(shader: &'a Shader, pos: Vector2<i32>, texture: u32) -> Self {
        let (mut VBO, mut VAO, mut EBO) = (0, 0, 0);

        unsafe {
            gl::GenVertexArrays(1, &mut VAO);
            gl::GenBuffers(1, &mut VBO);
            gl::GenBuffers(1, &mut EBO);

            gl::BindVertexArray(VAO);

            gl::BindBuffer(gl::ARRAY_BUFFER, VBO);

            let stride = 6 * mem::size_of::<GLfloat>() as GLsizei;
            // position attribute
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::EnableVertexAttribArray(0);
            // texture coord attribute
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * mem::size_of::<GLfloat>()) as *const c_void,
            );
            gl::EnableVertexAttribArray(1);

            gl::VertexAttribPointer(
                2,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (5 * mem::size_of::<GLfloat>()) as *const c_void,
            );
            gl::EnableVertexAttribArray(2);
        }

        Self {
            blocks: Box::new([[[BlockId::Air; CHUNKSIZE]; CHUNKHIEGHT]; CHUNKSIZE]),
            shader,
            VAO,
            VBO,
            EBO,
            texture,
            verts: Vec::with_capacity(CHUNKSIZE * CHUNKSIZE * CHUNKHIEGHT * 4 * 6),
            vertexCount: 0,
            indexCount: 0,
            pos,
        }
    }

    pub fn set(&mut self, cord: Vector3<usize>, block: BlockId) {
        self.blocks[cord.x][cord.y][cord.z] = block;
    }

    pub fn remesh(&mut self, world: &World) {
        self.verts.clear();
        let mut inds: Vec<u32> = Vec::new();
        let mut next = 0u32;

        for x in 0..CHUNKSIZE {
            for y in 0..CHUNKHIEGHT {
                for z in 0..CHUNKSIZE {
                    let id = self.blocks[x][y][z];
                    if id == BlockId::Air {
                        continue;
                    }

                    for d in 0..6 {
                        let mut isEnd: bool = false;
                        let mut dx = DIRS[d][0] + x as i16;
                        let dy = DIRS[d][1] + y as i16;
                        let mut dz = DIRS[d][2] + z as i16;

                        if dy < 0 || dy >= (CHUNKHIEGHT as i16) {
                            isEnd = true;
                        }

                        if !isEnd {
                            // Work out which chunk (cx, cz) we should peek into
                            let mut cx = self.pos.x; // current chunk-coords (i32)
                            let mut cz = self.pos.y;

                            if dx < 0 {
                                cx -= 1; // step West
                                dx += CHUNKSIZE as i16; // wrap into [0, CHUNKSIZE-1]
                            } else if dx >= CHUNKSIZE as i16 {
                                cx += 1; // step East
                                dx -= CHUNKSIZE as i16;
                            }

                            if dz < 0 {
                                cz -= 1; // step North
                                dz += CHUNKSIZE as i16;
                            } else if dz >= CHUNKSIZE as i16 {
                                cz += 1; // step South
                                dz -= CHUNKSIZE as i16;
                            }

                            // Look up the block in whatever chunk we ended up in
                            if world.getBlockType(
                                Vector2 { x: cx, y: cz },
                                Vector3 {
                                    x: dx as usize,
                                    y: dy as usize,
                                    z: dz as usize,
                                },
                            ) == BlockId::Air
                            {
                                isEnd = true; // neighbour is air → expose this face
                            }
                        }

                        if isEnd {
                            let mut face: Vec<f32> = vec![];
                            face.extend(&vertices[(120 / 6 * d)..(120 / 6 * d) + 5]);
                            face.push(id as i32 as f32);
                            face.extend(&vertices[(120 / 6 * d) + 5..(120 / 6 * d) + 10]);
                            face.push(id as i32 as f32);
                            face.extend(&vertices[(120 / 6 * d) + 10..(120 / 6 * d) + 15]);
                            face.push(id as i32 as f32);
                            face.extend(&vertices[(120 / 6 * d) + 15..(120 / 6 * d) + 20]);
                            face.push(id as i32 as f32);

                            for indx in 0..face.len() {
                                if indx % 6 == 0 {
                                    face[indx] += 1. * x as f32;
                                }
                                if indx % 6 == 1 {
                                    face[indx] += (1. * y as f32) - CHUNKHIEGHT as f32;
                                }
                                if indx % 6 == 2 {
                                    face[indx] += 1. * z as f32;
                                }
                            }
                            self.verts.extend(face.iter().clone());
                            let indsSlice = &[next, next + 1, next + 2, next, next + 2, next + 3];
                            inds.extend_from_slice(indsSlice);
                            next += 4;
                        }
                    }
                }
            }
        }

        self.verts.shrink_to_fit();
        self.vertexCount = self.verts.len() as i32;
        self.indexCount = inds.len() as i32;

        unsafe {
            gl::BindVertexArray(self.VAO);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.VBO);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.verts.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &self.verts[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.EBO);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (inds.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &inds[0] as *const u32 as *const c_void,
                gl::STATIC_DRAW,
            );
        }
    }
    pub fn draw(&self, proj: &Matrix4<f32>, view: &Matrix4<f32>) {
        unsafe {
            self.shader.useProgram();
            self.shader.setMat4(c"projection", proj);
            self.shader.setMat4(c"view", view);

            let model = cgmath::Matrix4::<f32>::from_translation(Vector3 {
                x: (self.pos.x as f32 - (RENDERDISTANCE as f32 / 2.0)) * 16.0,
                y: 0.0,
                z: (self.pos.y as f32 - (RENDERDISTANCE as f32 / 2.0)) * 16.0,
            });
            self.shader.setMat4(c"model", &model);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D_ARRAY, self.texture);

            gl::BindVertexArray(self.VAO);
            gl::DrawElements(
                gl::TRIANGLES,
                self.indexCount,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
        }
    }
}
