
#version 330 core
out vec4 FragColor;

in vec2 TexCoord;
in float TexIndex;

// texture samplers
uniform sampler2DArray texture1;

void main()
{
	FragColor = texture(texture1, vec3(TexCoord, TexIndex));
}

