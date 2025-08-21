use cgmath::{Matrix4, Point1, Point2, Point3, Vector2};
use cgmath::{SquareMatrix, Vector3};
use crossbeam::{channel, thread};
use gl::types::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::{mem, ptr};

//local
use crate::Shader;
use crate::common::make_texture_array;

//settings
const CHUNKSIZE: usize = 16;
const CHUNKHIEGHT: usize = 128;
const RENDERDISTANCE: usize = 3;
const THREADS: usize = 8;
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

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct ChunkPos {
    x: i32,
    z: i32,
}

pub struct World<'a> {
    chunks: HashMap<ChunkPos, Chunk<'a>>,
}

impl<'a> World<'a> {
    pub fn new(shader: &'a Shader) -> Self {
        let texture = make_texture_array(
            &["src/textures/txDirt.png", "src/textures/txGrass.png"],
            shader,
        );
        let mut chunks = HashMap::new();
        for x in 0..RENDERDISTANCE as i32 {
            for z in 0..RENDERDISTANCE as i32 {
                let pos = ChunkPos {
                    x: x - RENDERDISTANCE as i32 / 2,
                    z: z - RENDERDISTANCE as i32 / 2,
                };
                chunks.insert(pos, Chunk::new(shader, pos, texture));
            }
        }
        Self { chunks }
    }

    fn fillChunk(chunk: &mut Chunk) {
        for x in 0..16 {
            for y in 0..CHUNKHIEGHT {
                for z in 0..16 {
                    if y >= 120 {
                        chunk.set(Vector3 { x, y, z }, BlockId::Grass);
                    } else {
                        chunk.set(Vector3 { x, y, z }, BlockId::Dirt);
                    }
                }
            }
        }
    }

    pub fn setAll(&mut self) {
        let (tx, rx) = channel::unbounded::<ChunkPos>();

        for key in self.chunks.keys().copied() {
            tx.send(key);
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
                        let chunk = chunks.get_mut(&job).expect("setAll(): failed to get chunk");
                        World::fillChunk(chunk);
                    }
                });
            }
        })
        .unwrap();
    }

    pub fn chunkRemeshAll(&mut self) {
        let mut jobs = Vec::<ChunkPos>::new();
        for key in self.chunks.keys().copied() {
            jobs.push(key);
        }

        let (job_tx, job_rx) = channel::unbounded::<ChunkPos>();
        let (res_tx, res_rx) = channel::unbounded::<(ChunkPos, MeshData)>();

        for j in &jobs {
            job_tx.send(*j).unwrap();
        }
        drop(job_tx);

        thread::scope(|s| {
            let world_ref = &*self; // shared read-only borrow

            for _ in 0..THREADS {
                let job_rx = job_rx.clone();
                let res_tx = res_tx.clone();

                s.spawn(move |_| {
                    while let Ok(pos) = job_rx.recv() {
                        let chunk = &world_ref
                            .chunks
                            .get(&pos)
                            .expect("chunkRemeshAll(): couldnt find chunk"); // &Chunk
                        let mesh = chunk.remesh(world_ref); // read-only
                        res_tx.send((pos, mesh)).unwrap();
                    }
                });
            }
        })
        .unwrap(); // all workers have joined here

        // -------- Main thread: GPU upload & state updates --------
        while let Ok((pos, mesh)) = res_rx.try_recv() {
            let chunk = &mut self
                .chunks
                .get_mut(&pos)
                .expect("chunkRemeshAll(): couldnt find chunk"); // &mut borrow *after* scope
            chunk.uploadMesh(mesh);
        }
    }

    pub fn renderAll(&self, proj: &Matrix4<f32>, view: &Matrix4<f32>) {
        for chunk in self.chunks.values() {
            chunk.draw(proj, view);
        }
    }

    pub fn worldToLoc(pos: Point3<f32>) -> (Point3<i32>, ChunkPos) {
        let s = CHUNKSIZE as i32;

        // 1) go from world-space floats to integer block coords with floor semantics
        let wx = pos.x.floor() as i32;
        let wy = pos.y.floor() as i32;
        let wz = pos.z.floor() as i32;

        // 2) Euclidean chunk coords (work for negatives too)
        let cx = wx.div_euclid(s);
        let cz = wz.div_euclid(s);

        // 3) Non-negative local coords in [0, s)
        let lx = wx.rem_euclid(s);
        let lz = wz.rem_euclid(s);

        (Point3::new(lx, wy, lz), ChunkPos { x: cx, z: cz })
    }

    pub fn getBlockType(&self, pos: ChunkPos, blockPos: Point3<usize>) -> BlockId {
        match self.chunks.get(&pos) {
            Some(chunk) => chunk.blocks[blockPos.x][blockPos.y][blockPos.z],
            None => BlockId::Air,
        }
    }
}

pub struct MeshData {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
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
    // verts: Vec<f32>,
    // vertexCount: i32,
    indexCount: i32,
    pos: ChunkPos,
}

impl<'a> Chunk<'a> {
    pub fn new(shader: &'a Shader, pos: ChunkPos, texture: u32) -> Self {
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
            // verts: Vec::with_capacity(CHUNKSIZE * CHUNKSIZE * CHUNKHIEGHT * 4 * 6),
            // vertexCount: 0,
            indexCount: 0,
            pos,
        }
    }

    pub fn set(&mut self, cord: Vector3<usize>, block: BlockId) {
        self.blocks[cord.x][cord.y][cord.z] = block;
    }

    pub fn remesh(&self, world: &World) -> MeshData {
        let mut verts: Vec<f32> = Vec::with_capacity(CHUNKSIZE * CHUNKSIZE * CHUNKHIEGHT * 6);
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
                            let mut cz = self.pos.z;

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
                                ChunkPos { x: cx, z: cz },
                                Point3 {
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
                            verts.extend(face.iter().clone());
                            let indsSlice = &[next, next + 1, next + 2, next, next + 2, next + 3];
                            inds.extend_from_slice(indsSlice);
                            next += 4;
                        }
                    }
                }
            }
        }

        verts.shrink_to_fit();
        // self.vertexCount = self.verts.len() as i32;
        // self.indexCount = inds.len() as i32;

        MeshData {
            vertices: verts,
            indices: inds,
        }
    }

    pub fn uploadMesh(&mut self, data: MeshData) {
        self.indexCount = data.indices.len() as i32;
        unsafe {
            gl::BindVertexArray(self.VAO);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.VBO);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (data.vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &data.vertices[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.EBO);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (data.indices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &data.indices[0] as *const u32 as *const c_void,
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
                x: self.pos.x as f32 * 16.0 + 0.5,
                y: 0.0,
                z: self.pos.z as f32 * 16.0 + 0.5,
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
