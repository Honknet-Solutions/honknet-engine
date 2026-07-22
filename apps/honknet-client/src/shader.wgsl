struct Out{@builtin(position)position:vec4<f32>,@location(0)color:vec3<f32>};
@vertex fn vs(@location(0)p:vec2<f32>,@location(1)c:vec3<f32>)->Out{var o:Out;o.position=vec4(p,0.,1.);o.color=c;return o;}
@fragment fn fs(i:Out)->@location(0)vec4<f32>{return vec4(i.color,1.);}
