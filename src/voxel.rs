
#[allow(non_snake_case)]
struct voxelShader {
    shader: Shader,
}
impl voxelShader {
    fn new(vsPath: &str, fsPath: &str) -> Self {
        Self {
            shader: Shader::new(vsPath, fsPath),
        }
    }
}

#[allow(non_snake_case)]
struct Voxel<'a> {
    model: Matrix4<f32>,
    voxelShader: &'a voxelShader,
    texture: u32,
    VAO: u32,
    VBO: u32,
    EBO: u32,
}

#[allow(non_snake_case)]
impl<'a> Voxel<'a> {
    //creates a voxel
    fn new(tex_path: &str, voxelShader: &'a voxelShader) -> Self {
        let (mut VBO, mut VAO, mut EBO) = (0, 0, 0);
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

        const indices: [u32; 36] = [
            // back (‑Z)
            0, 1, 2, 0, 2, 3, // front (+Z)
            4, 6, 5, 4, 7, 6, // left (‑X)
            8, 9, 10, 8, 10, 11, // right (+X)
            12, 14, 13, 12, 15, 14, // bottom (‑Y)
            16, 17, 18, 16, 18, 19, // top (+Y)
            20, 22, 21, 20, 23, 22,
        ];

        //set up array buffers and buffer objects with the corresponding text cords and vertix
        //cords from vertices. Makes element buffer object bassed on indices
        unsafe {
            gl::GenVertexArrays(1, &mut VAO);
            gl::GenBuffers(1, &mut VBO);
            gl::GenBuffers(1, &mut EBO);

            gl::BindVertexArray(VAO);

            gl::BindBuffer(gl::ARRAY_BUFFER, VBO);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &vertices[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, EBO);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &indices[0] as *const u32 as *const c_void,
                gl::STATIC_DRAW,
            );

            let stride = 5 * mem::size_of::<GLfloat>() as GLsizei;
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
        }

        //set up textures for the shader.
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
            voxelShader.shader.useProgram(); // don't forget to activate/use the shader before setting uniforms!
            let texture1_name = c"texture1";
            voxelShader.shader.setInt(&texture1_name, 0);
        }

        Self {
            model: Matrix4::identity(),
            texture,
            voxelShader,
            VAO,
            VBO,
            EBO,
        }
    }
    fn bind(&self) {
        unsafe { gl::BindVertexArray(self.VAO) };
    }
    fn spawn(&self, cords: Vector3<i32>, projection: &Matrix4<f32>, view: &Matrix4<f32>) {
        unsafe {
            //TODO: setup texture changing for voxels
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture);

            self.voxelShader.shader.useProgram();
            self.voxelShader.shader.setMat4(c"projection", &projection);

            // camera/view transformation
            self.voxelShader.shader.setMat4(c"view", &view);

            let model = Matrix4::from_translation(Vector3::new(
                cords.x as f32,
                cords.y as f32,
                cords.z as f32,
            ));
            self.bind();
            self.transform(&model);
            gl::DrawElements(gl::TRIANGLES, 36, gl::UNSIGNED_INT, ptr::null());
        }
    }
    fn transform(&self, model: &Matrix4<f32>) {
        unsafe { self.voxelShader.shader.setMat4(c"model", &model) };
    }
}

impl Drop for Voxel<'_> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.VAO);
            gl::DeleteBuffers(1, &self.VBO);
            gl::DeleteBuffers(1, &self.EBO);
        }
    }
}
