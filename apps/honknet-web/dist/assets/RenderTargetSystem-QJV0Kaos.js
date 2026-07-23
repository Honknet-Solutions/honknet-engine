import{a as ze,j as T,G as He,w as te,c as u,s as Ct,M,e as re,l as V,t as St,B as se,x as G,F as Ve,S as kt,T as S,f as Q,y as xe,z as Fe,H as Le,J as Oe,K as We,d as F,L as wt,C as Y,D as ne,i as I,R as L,N as Mt,P as Bt,b as ge,O as _e,h as X,p as Rt,Q as Pt,W as At}from"./index-CWmGpe2x.js";import{f as Ut,e as Gt,a as be,b as z,G as $e,h as je,U as ie,d as It,F as Dt,S as Et,R as ae,B as zt}from"./Geometry-CmnrOTTD.js";class ve{constructor(e){typeof e=="number"?this.rawBinaryData=new ArrayBuffer(e):e instanceof Uint8Array?this.rawBinaryData=e.buffer:this.rawBinaryData=e,this.uint32View=new Uint32Array(this.rawBinaryData),this.float32View=new Float32Array(this.rawBinaryData),this.size=this.rawBinaryData.byteLength}get int8View(){return this._int8View||(this._int8View=new Int8Array(this.rawBinaryData)),this._int8View}get uint8View(){return this._uint8View||(this._uint8View=new Uint8Array(this.rawBinaryData)),this._uint8View}get int16View(){return this._int16View||(this._int16View=new Int16Array(this.rawBinaryData)),this._int16View}get int32View(){return this._int32View||(this._int32View=new Int32Array(this.rawBinaryData)),this._int32View}get float64View(){return this._float64Array||(this._float64Array=new Float64Array(this.rawBinaryData)),this._float64Array}get bigUint64View(){return this._bigUint64Array||(this._bigUint64Array=new BigUint64Array(this.rawBinaryData)),this._bigUint64Array}view(e){return this[`${e}View`]}destroy(){this.rawBinaryData=null,this.uint32View=null,this.float32View=null,this.uint16View=null,this._int8View=null,this._uint8View=null,this._int16View=null,this._int32View=null,this._float64Array=null,this._bigUint64Array=null}static sizeOf(e){switch(e){case"int8":case"uint8":return 1;case"int16":case"uint16":return 2;case"int32":case"uint32":case"float32":return 4;default:throw new Error(`${e} isn't a valid view type`)}}}function ye(n,e,t,r){if(t??(t=0),r??(r=Math.min(n.byteLength-t,e.byteLength)),!(t&7)&&!(r&7)){const s=r/8;new Float64Array(e,0,s).set(new Float64Array(n,t,s))}else if(!(t&3)&&!(r&3)){const s=r/4;new Float32Array(e,0,s).set(new Float32Array(n,t,s))}else new Uint8Array(e).set(new Uint8Array(n,t,r))}const Ht={normal:"normal-npm",add:"add-npm",screen:"screen-npm"};var k=(n=>(n[n.DISABLED=0]="DISABLED",n[n.RENDERING_MASK_ADD=1]="RENDERING_MASK_ADD",n[n.MASK_ACTIVE=2]="MASK_ACTIVE",n[n.INVERSE_MASK_ACTIVE=3]="INVERSE_MASK_ACTIVE",n[n.RENDERING_MASK_REMOVE=4]="RENDERING_MASK_REMOVE",n[n.NONE=5]="NONE",n))(k||{});function Te(n,e){return e.alphaMode==="no-premultiply-alpha"&&Ht[n]||n}const Vt=["precision mediump float;","void main(void){","float test = 0.1;","%forloop%","gl_FragColor = vec4(0.0);","}"].join(`
`);function Ft(n){let e="";for(let t=0;t<n;++t)t>0&&(e+=`
else `),t<n-1&&(e+=`if(test == ${t}.0){}`);return e}function Lt(n,e){if(n===0)throw new Error("Invalid value of `0` passed to `checkMaxIfStatementsInShader`");const t=e.createShader(e.FRAGMENT_SHADER);try{for(;;){const r=Vt.replace(/%forloop%/gi,Ft(n));if(e.shaderSource(t,r),e.compileShader(t),!e.getShaderParameter(t,e.COMPILE_STATUS))n=n/2|0;else break}}finally{e.deleteShader(t)}return n}let R=null;function Ot(){var e;if(R)return R;const n=Ut();return R=n.getParameter(n.MAX_TEXTURE_IMAGE_UNITS),R=Lt(R,n),(e=n.getExtension("WEBGL_lose_context"))==null||e.loseContext(),R}class Wt{constructor(){this.ids=Object.create(null),this.textures=[],this.count=0}clear(){for(let e=0;e<this.count;e++){const t=this.textures[e];this.textures[e]=null,this.ids[t.uid]=null}this.count=0}}class $t{constructor(){this.renderPipeId="batch",this.action="startBatch",this.start=0,this.size=0,this.textures=new Wt,this.blendMode="normal",this.topology="triangle-strip",this.canBundle=!0}destroy(){this.textures=null,this.gpuBindGroup=null,this.bindGroup=null,this.batcher=null,this.elements=null}}const D=[];let O=0;He.register({clear:()=>{if(D.length>0)for(const n of D)n&&n.destroy();D.length=0,O=0}});function Ce(){return O>0?D[--O]:new $t}function Se(n){n.elements=null,D[O++]=n}let A=0;const Ne=class Ke{constructor(e){this.uid=ze("batcher"),this.dirty=!0,this.batchIndex=0,this.batches=[],this._elements=[],e={...Ke.defaultOptions,...e},e.maxTextures||(T("v8.8.0","maxTextures is a required option for Batcher now, please pass it in the options"),e.maxTextures=Ot());const{maxTextures:t,attributesInitialSize:r,indicesInitialSize:s}=e;this.attributeBuffer=new ve(r*4),this.indexBuffer=new Uint16Array(s),this.maxTextures=t}begin(){this.elementSize=0,this.elementStart=0,this.indexSize=0,this.attributeSize=0;for(let e=0;e<this.batchIndex;e++)Se(this.batches[e]);this.batchIndex=0,this._batchIndexStart=0,this._batchIndexSize=0,this.dirty=!0}add(e){this._elements[this.elementSize++]=e,e._indexStart=this.indexSize,e._attributeStart=this.attributeSize,e._batcher=this,this.indexSize+=e.indexSize,this.attributeSize+=e.attributeSize*this.vertexSize}checkAndUpdateTexture(e,t){const r=e._batch.textures.ids[t._source.uid];return!r&&r!==0?!1:(e._textureId=r,e.texture=t,!0)}updateElement(e){this.dirty=!0;const t=this.attributeBuffer;e.packAsQuad?this.packQuadAttributes(e,t.float32View,t.uint32View,e._attributeStart,e._textureId):this.packAttributes(e,t.float32View,t.uint32View,e._attributeStart,e._textureId)}break(e){const t=this._elements;if(!t[this.elementStart])return;let r=Ce(),s=r.textures;s.clear();const i=t[this.elementStart];let a=Te(i.blendMode,i.texture._source),o=i.topology;this.attributeSize*4>this.attributeBuffer.size&&this._resizeAttributeBuffer(this.attributeSize*4),this.indexSize>this.indexBuffer.length&&this._resizeIndexBuffer(this.indexSize);const l=this.attributeBuffer.float32View,d=this.attributeBuffer.uint32View,f=this.indexBuffer;let c=this._batchIndexSize,x=this._batchIndexStart,g="startBatch",m=[];const v=this.maxTextures;for(let _=this.elementStart;_<this.elementSize;++_){const h=t[_];t[_]=null;const p=h.texture._source,y=Te(h.blendMode,p),b=a!==y||o!==h.topology;if(p._batchTick===A&&!b){h._textureId=p._textureBindLocation,c+=h.indexSize,h.packAsQuad?(this.packQuadAttributes(h,l,d,h._attributeStart,h._textureId),this.packQuadIndex(f,h._indexStart,h._attributeStart/this.vertexSize)):(this.packAttributes(h,l,d,h._attributeStart,h._textureId),this.packIndex(h,f,h._indexStart,h._attributeStart/this.vertexSize)),h._batch=r,m.push(h);continue}p._batchTick=A,(s.count>=v||b)&&(this._finishBatch(r,x,c-x,s,a,o,e,g,m),g="renderBatch",x=c,a=y,o=h.topology,r=Ce(),s=r.textures,s.clear(),m=[],++A),h._textureId=p._textureBindLocation=s.count,s.ids[p.uid]=s.count,s.textures[s.count++]=p,h._batch=r,m.push(h),c+=h.indexSize,h.packAsQuad?(this.packQuadAttributes(h,l,d,h._attributeStart,h._textureId),this.packQuadIndex(f,h._indexStart,h._attributeStart/this.vertexSize)):(this.packAttributes(h,l,d,h._attributeStart,h._textureId),this.packIndex(h,f,h._indexStart,h._attributeStart/this.vertexSize))}s.count>0&&(this._finishBatch(r,x,c-x,s,a,o,e,g,m),x=c,++A),this.elementStart=this.elementSize,this._batchIndexStart=x,this._batchIndexSize=c}_finishBatch(e,t,r,s,i,a,o,l,d){e.gpuBindGroup=null,e.bindGroup=null,e.action=l,e.batcher=this,e.textures=s,e.blendMode=i,e.topology=a,e.start=t,e.size=r,e.elements=d,++A,this.batches[this.batchIndex++]=e,o.add(e)}finish(e){this.break(e)}ensureAttributeBuffer(e){e*4<=this.attributeBuffer.size||this._resizeAttributeBuffer(e*4)}ensureIndexBuffer(e){e<=this.indexBuffer.length||this._resizeIndexBuffer(e)}_resizeAttributeBuffer(e){const t=Math.max(e,this.attributeBuffer.size*2),r=new ve(t);ye(this.attributeBuffer.rawBinaryData,r.rawBinaryData),this.attributeBuffer=r}_resizeIndexBuffer(e){const t=this.indexBuffer;let r=Math.max(e,t.length*1.5);r+=r%2;const s=r>65535?new Uint32Array(r):new Uint16Array(r);if(s.BYTES_PER_ELEMENT!==t.BYTES_PER_ELEMENT)for(let i=0;i<t.length;i++)s[i]=t[i];else ye(t.buffer,s.buffer);this.indexBuffer=s}packQuadIndex(e,t,r){e[t]=r+0,e[t+1]=r+1,e[t+2]=r+2,e[t+3]=r+0,e[t+4]=r+2,e[t+5]=r+3}packIndex(e,t,r,s){const i=e.indices,a=e.indexSize,o=e.indexOffset,l=e.attributeOffset;for(let d=0;d<a;d++)t[r++]=s+i[d+o]-l}destroy(e={}){var t;if(this.batches!==null){for(let r=0;r<this.batchIndex;r++)Se(this.batches[r]);this.batches=null,this.geometry.destroy(!0),this.geometry=null,e.shader&&((t=this.shader)==null||t.destroy(),this.shader=null);for(let r=0;r<this._elements.length;r++)this._elements[r]&&(this._elements[r]._batch=null);this._elements=null,this.indexBuffer=null,this.attributeBuffer.destroy(),this.attributeBuffer=null}}};Ne.defaultOptions={maxTextures:null,attributesInitialSize:4,indicesInitialSize:6};let jt=Ne;const Nt=new Float32Array(1),Kt=new Uint32Array(1);class qt extends Gt{constructor(){const t=new be({data:Nt,label:"attribute-batch-buffer",usage:z.VERTEX|z.COPY_DST,shrinkToFit:!1}),r=new be({data:Kt,label:"index-batch-buffer",usage:z.INDEX|z.COPY_DST,shrinkToFit:!1}),s=24;super({attributes:{aPosition:{buffer:t,format:"float32x2",stride:s,offset:0},aUV:{buffer:t,format:"float32x2",stride:s,offset:8},aColor:{buffer:t,format:"unorm8x4",stride:s,offset:16},aTextureIdAndRound:{buffer:t,format:"uint16x2",stride:s,offset:20}},indexBuffer:r})}}function ke(n,e,t){if(n)for(const r in n){const s=r.toLocaleLowerCase(),i=e[s];if(i){let a=n[r];r==="header"&&(a=a.replace(/@in\s+[^;]+;\s*/g,"").replace(/@out\s+[^;]+;\s*/g,"")),t&&i.push(`//----${t}----//`),i.push(a)}else te(`${r} placement hook does not exist in shader`)}}const Qt=/\{\{(.*?)\}\}/g;function we(n){var r;const e={};return(((r=n.match(Qt))==null?void 0:r.map(s=>s.replace(/[{()}]/g,"")))??[]).forEach(s=>{e[s]=[]}),e}function Me(n,e){let t;const r=/@in\s+([^;]+);/g;for(;(t=r.exec(n))!==null;)e.push(t[1])}function Be(n,e,t=!1){const r=[];Me(e,r),n.forEach(o=>{o.header&&Me(o.header,r)});const s=r;t&&s.sort();const i=s.map((o,l)=>`       @location(${l}) ${o},`).join(`
`);let a=e.replace(/@in\s+[^;]+;\s*/g,"");return a=a.replace("{{in}}",`
${i}
`),a}function Re(n,e){let t;const r=/@out\s+([^;]+);/g;for(;(t=r.exec(n))!==null;)e.push(t[1])}function Yt(n){const t=/\b(\w+)\s*:/g.exec(n);return t?t[1]:""}function Xt(n){const e=/@.*?\s+/g;return n.replace(e,"")}function Jt(n,e){const t=[];Re(e,t),n.forEach(l=>{l.header&&Re(l.header,t)});let r=0;const s=t.sort().map(l=>l.indexOf("builtin")>-1?l:`@location(${r++}) ${l}`).join(`,
`),i=t.sort().map(l=>`       var ${Xt(l)};`).join(`
`),a=`return VSOutput(
            ${t.sort().map(l=>` ${Yt(l)}`).join(`,
`)});`;let o=e.replace(/@out\s+[^;]+;\s*/g,"");return o=o.replace("{{struct}}",`
${s}
`),o=o.replace("{{start}}",`
${i}
`),o=o.replace("{{return}}",`
${a}
`),o}function Pe(n,e){let t=n;for(const r in e){const s=e[r];s.join(`
`).length?t=t.replace(`{{${r}}}`,`//-----${r} START-----//
${s.join(`
`)}
//----${r} FINISH----//`):t=t.replace(`{{${r}}}`,"")}return t}const w=Object.create(null),j=new Map;let Zt=0;function er({template:n,bits:e}){const t=qe(n,e);if(w[t])return w[t];const{vertex:r,fragment:s}=rr(n,e);return w[t]=Qe(r,s,e),w[t]}function tr({template:n,bits:e}){const t=qe(n,e);return w[t]||(w[t]=Qe(n.vertex,n.fragment,e)),w[t]}function rr(n,e){const t=e.map(a=>a.vertex).filter(a=>!!a),r=e.map(a=>a.fragment).filter(a=>!!a);let s=Be(t,n.vertex,!0);s=Jt(t,s);const i=Be(r,n.fragment,!0);return{vertex:s,fragment:i}}function qe(n,e){return e.map(t=>(j.has(t)||j.set(t,Zt++),j.get(t))).sort((t,r)=>t-r).join("-")+n.vertex+n.fragment}function Qe(n,e,t){const r=we(n),s=we(e);return t.forEach(i=>{ke(i.vertex,r,i.name),ke(i.fragment,s,i.name)}),{vertex:Pe(n,r),fragment:Pe(e,s)}}const sr=`
    @in aPosition: vec2<f32>;
    @in aUV: vec2<f32>;

    @out @builtin(position) vPosition: vec4<f32>;
    @out vUV : vec2<f32>;
    @out vColor : vec4<f32>;

    {{header}}

    struct VSOutput {
        {{struct}}
    };

    @vertex
    fn main( {{in}} ) -> VSOutput {

        var worldTransformMatrix = globalUniforms.uWorldTransformMatrix;
        var modelMatrix = mat3x3<f32>(
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0
          );
        var position = aPosition;
        var uv = aUV;

        {{start}}

        vColor = vec4<f32>(1., 1., 1., 1.);

        {{main}}

        vUV = uv;

        var modelViewProjectionMatrix = globalUniforms.uProjectionMatrix * worldTransformMatrix * modelMatrix;

        vPosition =  vec4<f32>((modelViewProjectionMatrix *  vec3<f32>(position, 1.0)).xy, 0.0, 1.0);

        vColor *= globalUniforms.uWorldColorAlpha;

        {{end}}

        {{return}}
    };
`,nr=`
    @in vUV : vec2<f32>;
    @in vColor : vec4<f32>;

    {{header}}

    @fragment
    fn main(
        {{in}}
      ) -> @location(0) vec4<f32> {

        {{start}}

        var outColor:vec4<f32>;

        {{main}}

        var finalColor:vec4<f32> = outColor * vColor;

        {{end}}

        return finalColor;
      };
`,ir=`
    in vec2 aPosition;
    in vec2 aUV;

    out vec4 vColor;
    out vec2 vUV;

    {{header}}

    void main(void){

        mat3 worldTransformMatrix = uWorldTransformMatrix;
        mat3 modelMatrix = mat3(
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0
          );
        vec2 position = aPosition;
        vec2 uv = aUV;

        {{start}}

        vColor = vec4(1.);

        {{main}}

        vUV = uv;

        mat3 modelViewProjectionMatrix = uProjectionMatrix * worldTransformMatrix * modelMatrix;

        gl_Position = vec4((modelViewProjectionMatrix * vec3(position, 1.0)).xy, 0.0, 1.0);

        vColor *= uWorldColorAlpha;

        {{end}}
    }
`,ar=`

    in vec4 vColor;
    in vec2 vUV;

    out vec4 finalColor;

    {{header}}

    void main(void) {

        {{start}}

        vec4 outColor;

        {{main}}

        finalColor = outColor * vColor;

        {{end}}
    }
`,or={name:"global-uniforms-bit",vertex:{header:`
        struct GlobalUniforms {
            uProjectionMatrix:mat3x3<f32>,
            uWorldTransformMatrix:mat3x3<f32>,
            uWorldColorAlpha: vec4<f32>,
            uResolution: vec2<f32>,
        }

        @group(0) @binding(0) var<uniform> globalUniforms : GlobalUniforms;
        `}},lr={name:"global-uniforms-bit",vertex:{header:`
          uniform mat3 uProjectionMatrix;
          uniform mat3 uWorldTransformMatrix;
          uniform vec4 uWorldColorAlpha;
          uniform vec2 uResolution;
        `}};function ur({bits:n,name:e}){const t=er({template:{fragment:nr,vertex:sr},bits:[or,...n]});return je.from({name:e,vertex:{source:t.vertex,entryPoint:"main"},fragment:{source:t.fragment,entryPoint:"main"}})}function cr({bits:n,name:e}){return new $e({name:e,...tr({template:{vertex:ir,fragment:ar},bits:[lr,...n]})})}const dr={name:"color-bit",vertex:{header:`
            @in aColor: vec4<f32>;
        `,main:`
            vColor *= vec4<f32>(aColor.rgb * aColor.a, aColor.a);
        `}},hr={name:"color-bit",vertex:{header:`
            in vec4 aColor;
        `,main:`
            vColor *= vec4(aColor.rgb * aColor.a, aColor.a);
        `}},N={};function fr(n){const e=[];if(n===1)e.push("@group(1) @binding(0) var textureSource1: texture_2d<f32>;"),e.push("@group(1) @binding(1) var textureSampler1: sampler;");else{let t=0;for(let r=0;r<n;r++)e.push(`@group(1) @binding(${t++}) var textureSource${r+1}: texture_2d<f32>;`),e.push(`@group(1) @binding(${t++}) var textureSampler${r+1}: sampler;`)}return e.join(`
`)}function pr(n){const e=[];if(n===1)e.push("outColor = textureSampleGrad(textureSource1, textureSampler1, vUV, uvDx, uvDy);");else{e.push("switch vTextureId {");for(let t=0;t<n;t++)t===n-1?e.push("  default:{"):e.push(`  case ${t}:{`),e.push(`      outColor = textureSampleGrad(textureSource${t+1}, textureSampler${t+1}, vUV, uvDx, uvDy);`),e.push("      break;}");e.push("}")}return e.join(`
`)}function mr(n){return N[n]||(N[n]={name:"texture-batch-bit",vertex:{header:`
                @in aTextureIdAndRound: vec2<u32>;
                @out @interpolate(flat) vTextureId : u32;
            `,main:`
                vTextureId = aTextureIdAndRound.y;
            `,end:`
                if(aTextureIdAndRound.x == 1)
                {
                    vPosition = vec4<f32>(roundPixels(vPosition.xy, globalUniforms.uResolution), vPosition.zw);
                }
            `},fragment:{header:`
                @in @interpolate(flat) vTextureId: u32;

                ${fr(n)}
            `,main:`
                var uvDx = dpdx(vUV);
                var uvDy = dpdy(vUV);

                ${pr(n)}
            `}}),N[n]}const K={};function xr(n){const e=[];for(let t=0;t<n;t++)t>0&&e.push("else"),t<n-1&&e.push(`if(vTextureId < ${t}.5)`),e.push("{"),e.push(`	outColor = texture(uTextures[${t}], vUV);`),e.push("}");return e.join(`
`)}function gr(n){return K[n]||(K[n]={name:"texture-batch-bit",vertex:{header:`
                in vec2 aTextureIdAndRound;
                out float vTextureId;

            `,main:`
                vTextureId = aTextureIdAndRound.y;
            `,end:`
                if(aTextureIdAndRound.x == 1.)
                {
                    gl_Position.xy = roundPixels(gl_Position.xy, uResolution);
                }
            `},fragment:{header:`
                in float vTextureId;

                uniform sampler2D uTextures[${n}];

            `,main:`

                ${xr(n)}
            `}}),K[n]}const _r={name:"round-pixels-bit",vertex:{header:`
            fn roundPixels(position: vec2<f32>, targetSize: vec2<f32>) -> vec2<f32>
            {
                return (floor(((position * 0.5 + 0.5) * targetSize) + 0.5) / targetSize) * 2.0 - 1.0;
            }
        `}},br={name:"round-pixels-bit",vertex:{header:`
            vec2 roundPixels(vec2 position, vec2 targetSize)
            {
                return (floor(((position * 0.5 + 0.5) * targetSize) + 0.5) / targetSize) * 2.0 - 1.0;
            }
        `}},Ae={};function vr(n){let e=Ae[n];if(e)return e;const t=new Int32Array(n);for(let r=0;r<n;r++)t[r]=r;return e=Ae[n]=new ie({uTextures:{value:t,type:"i32",size:n}},{isStatic:!0}),e}class Ue extends It{constructor(e){const t=cr({name:"batch",bits:[hr,gr(e),br]}),r=ur({name:"batch",bits:[dr,mr(e),_r]});super({glProgram:t,gpuProgram:r,resources:{batchSamplers:vr(e)}}),this.maxTextures=e}}let U=null;const Ye=class Xe extends jt{constructor(e){super(e),this.geometry=new qt,this.name=Xe.extension.name,this.vertexSize=6,U??(U=new Ue(e.maxTextures)),this.shader=U}packAttributes(e,t,r,s,i){const a=i<<16|e.roundPixels&65535,o=e.transform,l=o.a,d=o.b,f=o.c,c=o.d,x=o.tx,g=o.ty,{positions:m,uvs:v}=e,_=e.color,h=e.attributeOffset,C=h+e.attributeSize;for(let p=h;p<C;p++){const y=p*2,b=m[y],B=m[y+1];t[s++]=l*b+f*B+x,t[s++]=c*B+d*b+g,t[s++]=v[y],t[s++]=v[y+1],r[s++]=_,r[s++]=a}}packQuadAttributes(e,t,r,s,i){const a=e.texture,o=e.transform,l=o.a,d=o.b,f=o.c,c=o.d,x=o.tx,g=o.ty,m=e.bounds,v=m.maxX,_=m.minX,h=m.maxY,C=m.minY,p=a.uvs,y=e.color,b=i<<16|e.roundPixels&65535;t[s+0]=l*_+f*C+x,t[s+1]=c*C+d*_+g,t[s+2]=p.x0,t[s+3]=p.y0,r[s+4]=y,r[s+5]=b,t[s+6]=l*v+f*C+x,t[s+7]=c*C+d*v+g,t[s+8]=p.x1,t[s+9]=p.y1,r[s+10]=y,r[s+11]=b,t[s+12]=l*v+f*h+x,t[s+13]=c*h+d*v+g,t[s+14]=p.x2,t[s+15]=p.y2,r[s+16]=y,r[s+17]=b,t[s+18]=l*_+f*h+x,t[s+19]=c*h+d*_+g,t[s+20]=p.x3,t[s+21]=p.y3,r[s+22]=y,r[s+23]=b}_updateMaxTextures(e){this.shader.maxTextures!==e&&(U=new Ue(e),this.shader=U)}destroy(){this.shader=null,super.destroy()}};Ye.extension={type:[u.Batcher],name:"default"};let Je=Ye;class qr{constructor(e){this.items=Object.create(null);const{renderer:t,type:r,onUnload:s,priority:i,name:a}=e;this._renderer=t,t.gc.addResourceHash(this,"items",r,i??0),this._onUnload=s,this.name=a}add(e){return this.items[e.uid]?!1:(this.items[e.uid]=e,e.once("unload",this.remove,this),e._gcLastUsed=this._renderer.gc.now,!0)}remove(e,...t){var s;if(!this.items[e.uid])return;const r=e._gpuData[this._renderer.uid];r&&((s=this._onUnload)==null||s.call(this,e,...t),r.destroy(),e._gpuData[this._renderer.uid]=null,this.items[e.uid]=null)}removeAll(...e){Object.values(this.items).forEach(t=>t&&this.remove(t,...e))}destroy(...e){this.removeAll(...e),this.items=Object.create(null),this._renderer=null,this._onUnload=null}}var yr=`in vec2 vMaskCoord;
in vec2 vTextureCoord;

uniform sampler2D uTexture;
uniform sampler2D uMaskTexture;

uniform float uAlpha;
uniform vec4 uMaskClamp;
uniform float uInverse;
uniform float uChannel;

out vec4 finalColor;

void main(void)
{
    float clip = step(3.5,
        step(uMaskClamp.x, vMaskCoord.x) +
        step(uMaskClamp.y, vMaskCoord.y) +
        step(vMaskCoord.x, uMaskClamp.z) +
        step(vMaskCoord.y, uMaskClamp.w));

    // TODO look into why this is needed
    float npmAlpha = uAlpha;
    vec4 original = texture(uTexture, vTextureCoord);
    vec4 masky = texture(uMaskTexture, vMaskCoord);

    float a;
    if (uChannel == 1.0) {
        a = masky.a * npmAlpha * clip;
    } else {
        float alphaMul = 1.0 - npmAlpha * (1.0 - masky.a);
        a = alphaMul * masky.r * npmAlpha * clip;
    }

    if (uInverse == 1.0) {
        a = 1.0 - a;
    }

    finalColor = original * a;
}
`,Tr=`in vec2 aPosition;

out vec2 vTextureCoord;
out vec2 vMaskCoord;


uniform vec4 uInputSize;
uniform vec4 uOutputFrame;
uniform vec4 uOutputTexture;
uniform mat3 uFilterMatrix;

vec4 filterVertexPosition(  vec2 aPosition )
{
    vec2 position = aPosition * uOutputFrame.zw + uOutputFrame.xy;
       
    position.x = position.x * (2.0 / uOutputTexture.x) - 1.0;
    position.y = position.y * (2.0*uOutputTexture.z / uOutputTexture.y) - uOutputTexture.z;

    return vec4(position, 0.0, 1.0);
}

vec2 filterTextureCoord(  vec2 aPosition )
{
    return aPosition * (uOutputFrame.zw * uInputSize.zw);
}

vec2 getFilterCoord( vec2 aPosition )
{
    return  ( uFilterMatrix * vec3( filterTextureCoord(aPosition), 1.0)  ).xy;
}   

void main(void)
{
    gl_Position = filterVertexPosition(aPosition);
    vTextureCoord = filterTextureCoord(aPosition);
    vMaskCoord = getFilterCoord(aPosition);
}
`,Ge=`struct GlobalFilterUniforms {
  uInputSize:vec4<f32>,
  uInputPixel:vec4<f32>,
  uInputClamp:vec4<f32>,
  uOutputFrame:vec4<f32>,
  uGlobalFrame:vec4<f32>,
  uOutputTexture:vec4<f32>,
};

struct MaskUniforms {
  uFilterMatrix:mat3x3<f32>,
  uMaskClamp:vec4<f32>,
  uAlpha:f32,
  uInverse:f32,
  uChannel:f32,
};

@group(0) @binding(0) var<uniform> gfu: GlobalFilterUniforms;
@group(0) @binding(1) var uTexture: texture_2d<f32>;
@group(0) @binding(2) var uSampler : sampler;

@group(1) @binding(0) var<uniform> filterUniforms : MaskUniforms;
@group(1) @binding(1) var uMaskTexture: texture_2d<f32>;

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv : vec2<f32>,
    @location(1) filterUv : vec2<f32>,
};

fn filterVertexPosition(aPosition:vec2<f32>) -> vec4<f32>
{
    var position = aPosition * gfu.uOutputFrame.zw + gfu.uOutputFrame.xy;

    position.x = position.x * (2.0 / gfu.uOutputTexture.x) - 1.0;
    position.y = position.y * (2.0*gfu.uOutputTexture.z / gfu.uOutputTexture.y) - gfu.uOutputTexture.z;

    return vec4(position, 0.0, 1.0);
}

fn filterTextureCoord( aPosition:vec2<f32> ) -> vec2<f32>
{
    return aPosition * (gfu.uOutputFrame.zw * gfu.uInputSize.zw);
}

fn globalTextureCoord( aPosition:vec2<f32> ) -> vec2<f32>
{
  return  (aPosition.xy / gfu.uGlobalFrame.zw) + (gfu.uGlobalFrame.xy / gfu.uGlobalFrame.zw);
}

fn getFilterCoord(aPosition:vec2<f32> ) -> vec2<f32>
{
  return ( filterUniforms.uFilterMatrix * vec3( filterTextureCoord(aPosition), 1.0)  ).xy;
}

fn getSize() -> vec2<f32>
{
  return gfu.uGlobalFrame.zw;
}

@vertex
fn mainVertex(
  @location(0) aPosition : vec2<f32>,
) -> VSOutput {
  return VSOutput(
   filterVertexPosition(aPosition),
   filterTextureCoord(aPosition),
   getFilterCoord(aPosition)
  );
}

@fragment
fn mainFragment(
  @location(0) uv: vec2<f32>,
  @location(1) filterUv: vec2<f32>,
  @builtin(position) position: vec4<f32>
) -> @location(0) vec4<f32> {

    var maskClamp = filterUniforms.uMaskClamp;
    var uAlpha = filterUniforms.uAlpha;

    var clip = step(3.5,
      step(maskClamp.x, filterUv.x) +
      step(maskClamp.y, filterUv.y) +
      step(filterUv.x, maskClamp.z) +
      step(filterUv.y, maskClamp.w));

    var mask = textureSample(uMaskTexture, uSampler, filterUv);
    var source = textureSample(uTexture, uSampler, uv);

    var a: f32;
    if (filterUniforms.uChannel == 1.0) {
        a = mask.a * uAlpha * clip;
    } else {
        var alphaMul = 1.0 - uAlpha * (1.0 - mask.a);
        a = alphaMul * mask.r * uAlpha * clip;
    }

    if (filterUniforms.uInverse == 1.0) {
        a = 1.0 - a;
    }

    return source * a;
}
`;class Cr extends Dt{constructor(e){const{sprite:t,...r}=e,s=new Ct(t.texture),i=new ie({uFilterMatrix:{value:new M,type:"mat3x3<f32>"},uMaskClamp:{value:s.uClampFrame,type:"vec4<f32>"},uAlpha:{value:1,type:"f32"},uInverse:{value:e.inverse?1:0,type:"f32"},uChannel:{value:e.channel==="alpha"?1:0,type:"f32"}}),a=je.from({vertex:{source:Ge,entryPoint:"mainVertex"},fragment:{source:Ge,entryPoint:"mainFragment"}}),o=$e.from({vertex:Tr,fragment:yr,name:"mask-filter"});super({...r,gpuProgram:a,glProgram:o,clipToViewport:!1,resources:{filterUniforms:i,uMaskTexture:t.texture.source}}),this.sprite=t,this._textureMatrix=s}set inverse(e){this.resources.filterUniforms.uniforms.uInverse=e?1:0}get inverse(){return this.resources.filterUniforms.uniforms.uInverse===1}set channel(e){this.resources.filterUniforms.uniforms.uChannel=e==="alpha"?1:0}get channel(){return this.resources.filterUniforms.uniforms.uChannel===1?"alpha":"red"}apply(e,t,r,s){this._textureMatrix.texture=this.sprite.texture,e.calculateSpriteMatrix(this.resources.filterUniforms.uniforms.uFilterMatrix,this.sprite).prepend(this._textureMatrix.mapCoord),this.resources.uMaskTexture=this.sprite.texture.source,e.applyFilter(this,t,r,s)}}function Sr(n,e,t){const r=(n>>24&255)/255;e[t++]=(n&255)/255*r,e[t++]=(n>>8&255)/255*r,e[t++]=(n>>16&255)/255*r,e[t++]=r}class Ze{constructor(){this.batcherName="default",this.topology="triangle-list",this.attributeSize=4,this.indexSize=6,this.packAsQuad=!0,this.roundPixels=0,this._attributeStart=0,this._batcher=null,this._batch=null}get blendMode(){return this.renderable.groupBlendMode}get color(){return this.renderable.groupColorAlpha}reset(){this.renderable=null,this.texture=null,this._batcher=null,this._batch=null,this.bounds=null}destroy(){this.reset()}}const oe=class et{constructor(e,t){var r,s;this.state=Et.for2d(),this._batchersByInstructionSet=Object.create(null),this._activeBatches=Object.create(null),this.renderer=e,this._adaptor=t,(s=(r=this._adaptor).init)==null||s.call(r,this)}static getBatcher(e,t){return new this._availableBatchers[e]({maxTextures:t})}buildStart(e){let t=this._batchersByInstructionSet[e.uid];t||(t=this._batchersByInstructionSet[e.uid]=Object.create(null),t.default||(t.default=new Je({maxTextures:this.renderer.limits.maxBatchableTextures}))),this._activeBatches=t,this._activeBatch=this._activeBatches.default;for(const r in this._activeBatches)this._activeBatches[r].begin()}addToBatch(e,t){if(this._activeBatch.name!==e.batcherName){this._activeBatch.break(t);let r=this._activeBatches[e.batcherName];r||(r=this._activeBatches[e.batcherName]=et.getBatcher(e.batcherName,this.renderer.limits.maxBatchableTextures),r.begin()),this._activeBatch=r}this._activeBatch.add(e)}break(e){this._activeBatch.break(e)}buildEnd(e){this._activeBatch.break(e);const t=this._activeBatches;for(const r in t){const s=t[r],i=s.geometry;i.indexBuffer.setDataWithSize(s.indexBuffer,s.indexSize,!0),i.buffers[0].setDataWithSize(s.attributeBuffer.float32View,s.attributeSize,!1)}}upload(e){const t=this._batchersByInstructionSet[e.uid];for(const r in t){const s=t[r],i=s.geometry;s.dirty&&(s.dirty=!1,i.buffers[0].update(s.attributeSize*4))}}execute(e){if(e.action==="startBatch"){const t=e.batcher,r=t.geometry,s=t.shader;this._adaptor.start(this,r,s)}this._adaptor.execute(this,e)}destroy(){this.state=null,this.renderer=null,this._adaptor=null;for(const e in this._activeBatches)this._activeBatches[e].destroy();this._activeBatches=null}};oe.extension={type:[u.WebGLPipes,u.WebGPUPipes,u.CanvasPipes],name:"batch"};oe._availableBatchers=Object.create(null);let tt=oe;re.handleByMap(u.Batcher,tt._availableBatchers);re.add(Je);const kr=new se;class wr extends Ve{constructor(){super(),this.filters=[new Cr({sprite:new kt(S.EMPTY),inverse:!1,resolution:"inherit",antialias:"inherit"})]}get sprite(){return this.filters[0].sprite}set sprite(e){this.filters[0].sprite=e}get inverse(){return this.filters[0].inverse}set inverse(e){this.filters[0].inverse=e}get channel(){return this.filters[0].channel}set channel(e){this.filters[0].channel=e}}class rt{constructor(e){this._activeMaskStage=[],this._renderer=e}push(e,t,r){const s=this._renderer;if(s.renderPipes.batch.break(r),r.add({renderPipeId:"alphaMask",action:"pushMaskBegin",mask:e,inverse:t._maskOptions.inverse,canBundle:!1,maskedContainer:t}),e.inverse=t._maskOptions.inverse,e.channel=t._maskOptions.channel??"red",e.renderMaskToTexture){const i=e.mask;i.includeInBuild=!0,i.collectRenderables(r,s,null),i.includeInBuild=!1}s.renderPipes.batch.break(r),r.add({renderPipeId:"alphaMask",action:"pushMaskEnd",mask:e,maskedContainer:t,inverse:t._maskOptions.inverse,canBundle:!1})}pop(e,t,r){this._renderer.renderPipes.batch.break(r),r.add({renderPipeId:"alphaMask",action:"popMaskEnd",mask:e,inverse:t._maskOptions.inverse,canBundle:!1})}execute(e){const t=this._renderer,r=e.mask.renderMaskToTexture;if(e.action==="pushMaskBegin"){const s=V.get(wr);if(s.inverse=e.inverse,s.channel=e.mask.channel,r){e.mask.mask.measurable=!0;const i=St(e.mask.mask,!0,kr);e.mask.mask.measurable=!1,i.ceil();const a=t.renderTarget.renderTarget.colorTexture.source,o=G.getOptimalTexture(i.width,i.height,a._resolution,a.antialias);t.renderTarget.push(o,!0),t.globalUniforms.push({offset:i,worldColor:4294967295});const l=s.sprite;l.texture=o,l.worldTransform.tx=i.minX,l.worldTransform.ty=i.minY,this._activeMaskStage.push({filterEffect:s,maskedContainer:e.maskedContainer,filterTexture:o})}else s.sprite=e.mask.mask,this._activeMaskStage.push({filterEffect:s,maskedContainer:e.maskedContainer})}else if(e.action==="pushMaskEnd"){const s=this._activeMaskStage[this._activeMaskStage.length-1];r&&(t.type===ae.WEBGL&&t.renderTarget.finishRenderPass(),t.renderTarget.pop(),t.globalUniforms.pop()),t.filter.push({renderPipeId:"filter",action:"pushFilter",container:s.maskedContainer,filterEffect:s.filterEffect,canBundle:!1})}else if(e.action==="popMaskEnd"){t.filter.pop();const s=this._activeMaskStage.pop();r&&G.returnTexture(s.filterTexture),V.return(s.filterEffect)}}destroy(){this._renderer=null,this._activeMaskStage=null}}rt.extension={type:[u.WebGLPipes,u.WebGPUPipes,u.CanvasPipes],name:"alphaMask"};class st{constructor(e){this._colorStack=[],this._colorStackIndex=0,this._currentColor=0,this._renderer=e}buildStart(){this._colorStack[0]=15,this._colorStackIndex=1,this._currentColor=15}push(e,t,r){this._renderer.renderPipes.batch.break(r);const i=this._colorStack;i[this._colorStackIndex]=i[this._colorStackIndex-1]&e.mask;const a=this._colorStack[this._colorStackIndex];a!==this._currentColor&&(this._currentColor=a,r.add({renderPipeId:"colorMask",colorMask:a,canBundle:!1})),this._colorStackIndex++}pop(e,t,r){this._renderer.renderPipes.batch.break(r);const i=this._colorStack;this._colorStackIndex--;const a=i[this._colorStackIndex-1];a!==this._currentColor&&(this._currentColor=a,r.add({renderPipeId:"colorMask",colorMask:a,canBundle:!1}))}execute(e){this._renderer.colorMask.setMask(e.colorMask)}destroy(){this._renderer=null,this._colorStack=null}}st.extension={type:[u.WebGLPipes,u.WebGPUPipes],name:"colorMask"};class nt{constructor(e){this._maskStackHash={},this._maskHash=new WeakMap,this._renderer=e}push(e,t,r){var s;const i=e,a=this._renderer;a.renderPipes.batch.break(r),a.renderPipes.blendMode.setBlendMode(i.mask,"none",r),r.add({renderPipeId:"stencilMask",action:"pushMaskBegin",mask:e,inverse:t._maskOptions.inverse,canBundle:!1});const o=i.mask;o.includeInBuild=!0,this._maskHash.has(i)||this._maskHash.set(i,{instructionsStart:0,instructionsLength:0});const l=this._maskHash.get(i);l.instructionsStart=r.instructionSize,o.collectRenderables(r,a,null),o.includeInBuild=!1,a.renderPipes.batch.break(r),r.add({renderPipeId:"stencilMask",action:"pushMaskEnd",mask:e,inverse:t._maskOptions.inverse,canBundle:!1});const d=r.instructionSize-l.instructionsStart-1;l.instructionsLength=d;const f=a.renderTarget.renderTarget.uid;(s=this._maskStackHash)[f]??(s[f]=0)}pop(e,t,r){const s=e,i=this._renderer;i.renderPipes.batch.break(r),i.renderPipes.blendMode.setBlendMode(s.mask,"none",r),r.add({renderPipeId:"stencilMask",action:"popMaskBegin",inverse:t._maskOptions.inverse,canBundle:!1});const a=this._maskHash.get(e);for(let o=0;o<a.instructionsLength;o++)r.instructions[r.instructionSize++]=r.instructions[a.instructionsStart++];r.add({renderPipeId:"stencilMask",action:"popMaskEnd",canBundle:!1})}execute(e){var t;const r=this._renderer,s=r,i=r.renderTarget.renderTarget.uid;let a=(t=this._maskStackHash)[i]??(t[i]=0);e.action==="pushMaskBegin"?(s.renderTarget.ensureDepthStencil(),s.stencil.setStencilMode(k.RENDERING_MASK_ADD,a),a++,s.colorMask.setMask(0)):e.action==="pushMaskEnd"?(e.inverse?s.stencil.setStencilMode(k.INVERSE_MASK_ACTIVE,a):s.stencil.setStencilMode(k.MASK_ACTIVE,a),s.colorMask.setMask(15)):e.action==="popMaskBegin"?(s.colorMask.setMask(0),a!==0?s.stencil.setStencilMode(k.RENDERING_MASK_REMOVE,a):(s.renderTarget.clear(null,Q.STENCIL),s.stencil.setStencilMode(k.DISABLED,a)),a--):e.action==="popMaskEnd"&&(e.inverse?s.stencil.setStencilMode(k.INVERSE_MASK_ACTIVE,a):s.stencil.setStencilMode(k.MASK_ACTIVE,a),s.colorMask.setMask(15)),this._maskStackHash[i]=a}destroy(){this._renderer=null,this._maskStackHash=null,this._maskHash=null}}nt.extension={type:[u.WebGLPipes,u.WebGPUPipes],name:"stencilMask"};class it{constructor(e){this._renderer=e}updateRenderable(){}destroyRenderable(){}validateRenderable(){return!1}addRenderable(e,t){this._renderer.renderPipes.batch.break(t),t.add(e)}execute(e){e.isRenderable&&e.render(this._renderer)}destroy(){this._renderer=null}}it.extension={type:[u.WebGLPipes,u.WebGPUPipes,u.CanvasPipes],name:"customRender"};function J(n,e){const t=n.instructionSet,r=t.instructions;for(let s=0;s<t.instructionSize;s++){const i=r[s];e[i.renderPipeId].execute(i)}}class at{constructor(e){this._renderer=e}addRenderGroup(e,t){e.isCachedAsTexture?this._addRenderableCacheAsTexture(e,t):this._addRenderableDirect(e,t)}execute(e){e.isRenderable&&(e.isCachedAsTexture?this._executeCacheAsTexture(e):this._executeDirect(e))}destroy(){this._renderer=null}_addRenderableDirect(e,t){this._renderer.renderPipes.batch.break(t),e._batchableRenderGroup&&(V.return(e._batchableRenderGroup),e._batchableRenderGroup=null),t.add(e)}_addRenderableCacheAsTexture(e,t){const r=e._batchableRenderGroup??(e._batchableRenderGroup=V.get(Ze));r.renderable=e.root,r.transform=e.root.relativeGroupTransform,r.texture=e.texture,r.bounds=e._textureBounds,t.add(e),this._renderer.renderPipes.blendMode.pushBlendMode(e,e.root.groupBlendMode,t),this._renderer.renderPipes.batch.addToBatch(r,t),this._renderer.renderPipes.blendMode.popBlendMode(t)}_executeCacheAsTexture(e){if(e.textureNeedsUpdate){e.textureNeedsUpdate=!1;const t=new M().translate(-e._textureBounds.x,-e._textureBounds.y);this._renderer.renderTarget.push(e.texture,!0,null,e.texture.frame),this._renderer.globalUniforms.push({worldTransformMatrix:t,worldColor:4294967295,offset:{x:0,y:0}}),J(e,this._renderer.renderPipes),this._renderer.renderTarget.finishRenderPass(),this._renderer.renderTarget.pop(),this._renderer.globalUniforms.pop()}e._batchableRenderGroup._batcher.updateElement(e._batchableRenderGroup),e._batchableRenderGroup._batcher.geometry.buffers[0].update()}_executeDirect(e){this._renderer.globalUniforms.push({worldTransformMatrix:e.inverseParentTextureTransform,worldColor:e.worldColorAlpha}),J(e,this._renderer.renderPipes),this._renderer.globalUniforms.pop()}}at.extension={type:[u.WebGLPipes,u.WebGPUPipes,u.CanvasPipes],name:"renderGroup"};class ot{constructor(e){this._renderer=e}addRenderable(e,t){const r=this._getGpuSprite(e);e.didViewUpdate&&this._updateBatchableSprite(e,r),this._renderer.renderPipes.batch.addToBatch(r,t)}updateRenderable(e){const t=this._getGpuSprite(e);e.didViewUpdate&&this._updateBatchableSprite(e,t),t._batcher.updateElement(t)}validateRenderable(e){const t=this._getGpuSprite(e);return!t._batcher.checkAndUpdateTexture(t,e._texture)}_updateBatchableSprite(e,t){t.bounds=e.visualBounds,t.texture=e._texture}_getGpuSprite(e){return e._gpuData[this._renderer.uid]||this._initGPUSprite(e)}_initGPUSprite(e){const t=new Ze;return t.renderable=e,t.transform=e.groupTransform,t.texture=e._texture,t.bounds=e.visualBounds,t.roundPixels=this._renderer._roundPixels|e._roundPixels,e._gpuData[this._renderer.uid]=t,t}destroy(){this._renderer=null}}ot.extension={type:[u.WebGLPipes,u.WebGPUPipes,u.CanvasPipes],name:"sprite"};const E={};re.handle(u.BlendMode,n=>{if(!n.name)throw new Error("BlendMode extension must have a name property");E[n.name]=n.ref},n=>{delete E[n.name]});class lt{constructor(e){this._blendModeStack=[],this._isAdvanced=!1,this._filterHash=Object.create(null),this._renderer=e,this._renderer.runners.prerender.add(this)}prerender(){this._activeBlendMode="normal",this._isAdvanced=!1}pushBlendMode(e,t,r){this._blendModeStack.push(t),this.setBlendMode(e,t,r)}popBlendMode(e){this._blendModeStack.pop();const t=this._blendModeStack[this._activeBlendMode.length-1]??"normal";this.setBlendMode(null,t,e)}setBlendMode(e,t,r){var i;const s=e instanceof xe;if(this._activeBlendMode===t){this._isAdvanced&&e&&!s&&((i=this._renderableList)==null||i.push(e));return}this._isAdvanced&&this._endAdvancedBlendMode(r),this._activeBlendMode=t,e&&(this._isAdvanced=!!E[t],this._isAdvanced&&this._beginAdvancedBlendMode(e,r))}_beginAdvancedBlendMode(e,t){this._renderer.renderPipes.batch.break(t);const r=this._activeBlendMode;if(!E[r]){te(`Unable to assign BlendMode: '${r}'. You may want to include: import 'pixi.js/advanced-blend-modes'`);return}const s=this._ensureFilterEffect(r),i=e instanceof xe,a={renderPipeId:"filter",action:"pushFilter",filterEffect:s,renderables:i?null:[e],container:i?e.root:null,canBundle:!1};this._renderableList=a.renderables,t.add(a)}_ensureFilterEffect(e){let t=this._filterHash[e];return t||(t=this._filterHash[e]=new Ve,t.filters=[new E[e]]),t}_endAdvancedBlendMode(e){this._isAdvanced=!1,this._renderableList=null,this._renderer.renderPipes.batch.break(e),e.add({renderPipeId:"filter",action:"popFilter",canBundle:!1})}buildStart(){this._isAdvanced=!1}buildEnd(e){this._isAdvanced&&this._endAdvancedBlendMode(e)}destroy(){this._renderer=null,this._renderableList=null;for(const e in this._filterHash)this._filterHash[e].destroy();this._filterHash=null}}lt.extension={type:[u.WebGLPipes,u.WebGPUPipes,u.CanvasPipes],name:"blendMode"};function Z(n,e){e||(e=0);for(let t=e;t<n.length&&n[t];t++)n[t]=null}const Mr=new F,Ie=Le|Oe|We;function ut(n,e=!1){Br(n);const t=n.childrenToUpdate,r=n.updateTick++;for(const s in t){const i=Number(s),a=t[s],o=a.list,l=a.index;for(let d=0;d<l;d++){const f=o[d];f.parentRenderGroup===n&&f.relativeRenderGroupDepth===i&&ct(f,r,0)}Z(o,l),a.index=0}if(e)for(let s=0;s<n.renderGroupChildren.length;s++)ut(n.renderGroupChildren[s],e)}function Br(n){const e=n.root;let t;if(n.renderGroupParent){const r=n.renderGroupParent;n.worldTransform.appendFrom(e.relativeGroupTransform,r.worldTransform),n.worldColor=Fe(e.groupColor,r.worldColor),t=e.groupAlpha*r.worldAlpha}else n.worldTransform.copyFrom(e.localTransform),n.worldColor=e.localColor,t=e.localAlpha;t=t<0?0:t>1?1:t,n.worldAlpha=t,n.worldColorAlpha=n.worldColor+((t*255|0)<<24)}function ct(n,e,t){if(e===n.updateTick)return;n.updateTick=e,n.didChange=!1;const r=n.localTransform;n.updateLocalTransform();const s=n.parent;if(s&&!s.renderGroup?(t|=n._updateFlags,n.relativeGroupTransform.appendFrom(r,s.relativeGroupTransform),t&Ie&&De(n,s,t)):(t=n._updateFlags,n.relativeGroupTransform.copyFrom(r),t&Ie&&De(n,Mr,t)),!n.renderGroup){const i=n.children,a=i.length;for(let d=0;d<a;d++)ct(i[d],e,t);const o=n.parentRenderGroup,l=n;l.renderPipeId&&!o.structureDidChange&&o.updateRenderable(l)}}function De(n,e,t){if(t&Oe){n.groupColor=Fe(n.localColor,e.groupColor);let r=n.localAlpha*e.groupAlpha;r=r<0?0:r>1?1:r,n.groupAlpha=r,n.groupColorAlpha=n.groupColor+((r*255|0)<<24)}t&We&&(n.groupBlendMode=n.localBlendMode==="inherit"?e.groupBlendMode:n.localBlendMode),t&Le&&(n.globalDisplayStatus=n.localDisplayStatus&e.globalDisplayStatus),n._updateFlags=0}function Rr(n,e){const{list:t}=n.childrenRenderablesToUpdate;let r=!1;for(let s=0;s<n.childrenRenderablesToUpdate.index;s++){const i=t[s];if(r=e[i.renderPipeId].validateRenderable(i),r)break}return n.structureDidChange=r,r}const Pr=new M;class dt{constructor(e){this._renderer=e}render({container:e,transform:t}){const r=e.parent,s=e.renderGroup.renderGroupParent;e.parent=null,e.renderGroup.renderGroupParent=null;const i=this._renderer,a=Pr;t&&(a.copyFrom(e.renderGroup.localTransform),e.renderGroup.localTransform.copyFrom(t));const o=i.renderPipes;this._updateCachedRenderGroups(e.renderGroup,null),this._updateRenderGroups(e.renderGroup),i.globalUniforms.start({worldTransformMatrix:t?e.renderGroup.localTransform:e.renderGroup.worldTransform,worldColor:e.renderGroup.worldColorAlpha}),J(e.renderGroup,o),o.uniformBatch&&o.uniformBatch.renderEnd(),t&&e.renderGroup.localTransform.copyFrom(a),e.parent=r,e.renderGroup.renderGroupParent=s}destroy(){this._renderer=null}_updateCachedRenderGroups(e,t){if(e._parentCacheAsTextureRenderGroup=t,e.isCachedAsTexture){if(!e.textureNeedsUpdate)return;t=e}for(let r=e.renderGroupChildren.length-1;r>=0;r--)this._updateCachedRenderGroups(e.renderGroupChildren[r],t);if(e.invalidateMatrices(),e.isCachedAsTexture){if(e.textureNeedsUpdate){const r=e.root.getLocalBounds(),s=this._renderer,i=e.textureOptions.resolution||s.view.resolution,a=e.textureOptions.antialias??s.view.antialias,o=e.textureOptions.scaleMode??"linear",l=e.texture;r.ceil(),e.texture&&G.returnTexture(e.texture,!0);const d=G.getOptimalTexture(r.width,r.height,i,a);d._source.style=new wt({scaleMode:o}),e.texture=d,e._textureBounds||(e._textureBounds=new se),e._textureBounds.copyFrom(r),l!==e.texture&&e.renderGroupParent&&(e.renderGroupParent.structureDidChange=!0)}}else e.texture&&(G.returnTexture(e.texture,!0),e.texture=null)}_updateRenderGroups(e){const t=this._renderer,r=t.renderPipes;if(e.runOnRender(t),e.instructionSet.renderPipes=r,e.structureDidChange?Z(e.childrenRenderablesToUpdate.list,0):Rr(e,r),ut(e),e.structureDidChange?(e.structureDidChange=!1,this._buildInstructions(e,t)):this._updateRenderables(e),e.childrenRenderablesToUpdate.index=0,t.renderPipes.batch.upload(e.instructionSet),!(e.isCachedAsTexture&&!e.textureNeedsUpdate))for(let s=0;s<e.renderGroupChildren.length;s++)this._updateRenderGroups(e.renderGroupChildren[s])}_updateRenderables(e){const{list:t,index:r}=e.childrenRenderablesToUpdate;for(let s=0;s<r;s++){const i=t[s];i.didViewUpdate&&e.updateRenderable(i)}Z(t,r)}_buildInstructions(e,t){const r=e.root,s=e.instructionSet;s.reset();const i=t.renderPipes?t:t.batch.renderer,a=i.renderPipes;a.batch.buildStart(s),a.blendMode.buildStart(),a.colorMask.buildStart(),r.sortableChildren&&r.sortChildren(),r.collectRenderablesWithEffects(s,i,null),a.batch.buildEnd(s),a.blendMode.buildEnd(s)}}dt.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"renderGroup"};const le=class ht{constructor(){this.clearBeforeRender=!0,this._backgroundColor=new Y(0),this.color=this._backgroundColor,this.alpha=1}init(e){e={...ht.defaultOptions,...e},this.clearBeforeRender=e.clearBeforeRender,this.color=e.background||e.backgroundColor||this._backgroundColor,this.alpha=e.backgroundAlpha,this._backgroundColor.setAlpha(e.backgroundAlpha)}get color(){return this._backgroundColor}set color(e){Y.shared.setValue(e).alpha<1&&this._backgroundColor.alpha===1&&te("Cannot set a transparent background on an opaque canvas. To enable transparency, set backgroundAlpha < 1 when initializing your Application."),this._backgroundColor.setValue(e)}get alpha(){return this._backgroundColor.alpha}set alpha(e){this._backgroundColor.setAlpha(e)}get colorRgba(){return this._backgroundColor.toArray()}destroy(){}};le.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"background",priority:0};le.defaultOptions={backgroundAlpha:1,backgroundColor:0,clearBeforeRender:!0};let Ar=le;const q={png:"image/png",jpg:"image/jpeg",webp:"image/webp"},ue=class ft{constructor(e){this._renderer=e}_normalizeOptions(e,t={}){return e instanceof F||e instanceof S?{target:e,...t}:{...t,...e}}async image(e){const t=ne.get().createImage();return t.src=await this.base64(e),t}async base64(e){e=this._normalizeOptions(e,ft.defaultImageOptions);const{format:t,quality:r}=e,s=this.canvas(e);if(s.toBlob!==void 0)return new Promise((i,a)=>{s.toBlob(o=>{if(!o){a(new Error("ICanvas.toBlob failed!"));return}const l=new FileReader;l.onload=()=>i(l.result),l.onerror=a,l.readAsDataURL(o)},q[t],r)});if(s.toDataURL!==void 0)return s.toDataURL(q[t],r);if(s.convertToBlob!==void 0){const i=await s.convertToBlob({type:q[t],quality:r});return new Promise((a,o)=>{const l=new FileReader;l.onload=()=>a(l.result),l.onerror=o,l.readAsDataURL(i)})}throw new Error("Extract.base64() requires ICanvas.toDataURL, ICanvas.toBlob, or ICanvas.convertToBlob to be implemented")}canvas(e){e=this._normalizeOptions(e);const t=e.target,r=this._renderer;if(t instanceof S)return r.texture.generateCanvas(t);const s=r.textureGenerator.generateTexture(e),i=r.texture.generateCanvas(s);return s.destroy(!0),i}pixels(e){e=this._normalizeOptions(e);const t=e.target,r=this._renderer,s=t instanceof S?t:r.textureGenerator.generateTexture(e),i=r.texture.getPixels(s);return t instanceof F&&s.destroy(!0),i}texture(e){return e=this._normalizeOptions(e),e.target instanceof S?e.target:this._renderer.textureGenerator.generateTexture(e)}download(e){e=this._normalizeOptions(e);const t=this.canvas(e),r=document.createElement("a");r.download=e.filename??"image.png",r.href=t.toDataURL("image/png"),document.body.appendChild(r),r.click(),document.body.removeChild(r)}log(e){const t=e.width??200;e=this._normalizeOptions(e);const r=this.canvas(e),s=r.toDataURL();console.log(`[Pixi Texture] ${r.width}px ${r.height}px`);const i=["font-size: 1px;",`padding: ${t}px 300px;`,`background: url(${s}) no-repeat;`,"background-size: contain;"].join(" ");console.log("%c ",i)}destroy(){this._renderer=null}};ue.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"extract"};ue.defaultImageOptions={format:"png",quality:1};let Ur=ue;class ce extends S{static create(e){const{dynamic:t,textureOptions:r,...s}=e;return new ce({...r,source:new I(s),dynamic:t??!1})}resize(e,t,r){return this.source.resize(e,t,r),this}}const Gr=new L,Ir=new se,Dr=[0,0,0,0];class pt{constructor(e){this._renderer=e}generateTexture(e){var f;e instanceof F&&(e={target:e,frame:void 0,textureSourceOptions:{},resolution:void 0});const t=e.resolution||this._renderer.resolution,r=e.antialias||this._renderer.view.antialias,s=e.target;let i=e.clearColor;i?i=Array.isArray(i)&&i.length===4?i:Y.shared.setValue(i).toArray():i=Dr;const a=((f=e.frame)==null?void 0:f.copyTo(Gr))||Mt(s,Ir).rectangle,o=e.defaultAnchor&&{defaultAnchor:e.defaultAnchor};a.width=Math.max(a.width,1/t)|0,a.height=Math.max(a.height,1/t)|0;const l=ce.create({...e.textureSourceOptions,width:a.width,height:a.height,resolution:t,antialias:r,textureOptions:o}),d=M.shared.translate(-a.x,-a.y);return this._renderer.render({container:s,transform:d,target:l,clearColor:i}),l.source.updateMipmaps(),l}destroy(){this._renderer=null}}pt.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"textureGenerator"};function Er(n){let e=!1;for(const r in n)if(n[r]==null){e=!0;break}if(!e)return n;const t=Object.create(null);for(const r in n){const s=n[r];s&&(t[r]=s)}return t}function zr(n){let e=0;for(let t=0;t<n.length;t++)n[t]==null?e++:n[t-e]=n[t];return n.length-=e,n}const de=class mt{constructor(e){this._managedResources=[],this._managedResourceHashes=[],this._managedCollections=[],this._ready=!1,this._renderer=e}init(e){e={...mt.defaultOptions,...e},this.maxUnusedTime=e.gcMaxUnusedTime,this._frequency=e.gcFrequency,this.enabled=e.gcActive,this.now=performance.now()}get enabled(){return!!this._handler}set enabled(e){this.enabled!==e&&(e?(this._handler=this._renderer.scheduler.repeat(()=>{this._ready=!0},this._frequency,!1),this._collectionsHandler=this._renderer.scheduler.repeat(()=>{for(const t of this._managedCollections){const{context:r,collection:s,type:i}=t;i==="hash"?r[s]=Er(r[s]):r[s]=zr(r[s])}},this._frequency)):(this._renderer.scheduler.cancel(this._handler),this._renderer.scheduler.cancel(this._collectionsHandler),this._handler=0,this._collectionsHandler=0))}prerender({container:e}){this.now=performance.now(),e.renderGroup.gcTick=this._renderer.tick++,this._updateInstructionGCTick(e.renderGroup,e.renderGroup.gcTick)}postrender(){!this._ready||!this.enabled||(this.run(),this._ready=!1)}_updateInstructionGCTick(e,t){e.instructionSet.gcTick=t,e.gcTick=t;for(const r of e.renderGroupChildren)this._updateInstructionGCTick(r,t)}addCollection(e,t,r){this._managedCollections.push({context:e,collection:t,type:r})}addResource(e,t){var s,i;if(e._gcLastUsed!==-1){e._gcLastUsed=this.now,(s=e._onTouch)==null||s.call(e,this.now);return}const r=this._managedResources.length;e._gcData={index:r,type:t},e._gcLastUsed=this.now,(i=e._onTouch)==null||i.call(e,this.now),e.once("unload",this.removeResource,this),this._managedResources.push(e)}removeResource(e){const t=e._gcData;if(!t)return;const r=t.index,s=this._managedResources.length-1;if(r!==s){const i=this._managedResources[s];this._managedResources[r]=i,i._gcData.index=r}this._managedResources.length--,e._gcData=null,e._gcLastUsed=-1}addResourceHash(e,t,r,s=0){this._managedResourceHashes.push({context:e,hash:t,type:r,priority:s}),this._managedResourceHashes.sort((i,a)=>i.priority-a.priority)}run(){const e=performance.now(),t=this._managedResourceHashes;for(const s of t)this.runOnHash(s,e);let r=0;for(let s=0;s<this._managedResources.length;s++){const i=this._managedResources[s];r=this.runOnResource(i,e,r)}this._managedResources.length=r}updateRenderableGCTick(e,t){var i,a;const r=e.renderGroup??e.parentRenderGroup,s=((i=r==null?void 0:r.instructionSet)==null?void 0:i.gcTick)??-1;((r==null?void 0:r.gcTick)??0)===s&&(e._gcLastUsed=t,(a=e._onTouch)==null||a.call(e,t))}runOnResource(e,t,r){const s=e._gcData;return s.type==="renderable"&&this.updateRenderableGCTick(e,t),t-e._gcLastUsed<this.maxUnusedTime||!e.autoGarbageCollect?(this._managedResources[r]=e,s.index=r,r++):(e.unload(),e._gcData=null,e._gcLastUsed=-1,e.off("unload",this.removeResource,this)),r}_createHashClone(e,t){const r=Object.create(null);for(const s in e){if(s===t)break;e[s]!==null&&(r[s]=e[s])}return r}runOnHash(e,t){var d;const{context:r,hash:s,type:i}=e,a=r[s];let o=null,l=0;for(const f in a){const c=a[f];if(c===null){l++,l===1e4&&!o&&(o=this._createHashClone(a,f));continue}if(c._gcLastUsed===-1){c._gcLastUsed=t,(d=c._onTouch)==null||d.call(c,t),o&&(o[f]=c);continue}if(i==="renderable"&&this.updateRenderableGCTick(c,t),!(t-c._gcLastUsed<this.maxUnusedTime)&&c.autoGarbageCollect){if(i==="renderable"){const g=c,m=g.renderGroup??g.parentRenderGroup;m&&(m.structureDidChange=!0)}c.unload(),c._gcData=null,c._gcLastUsed=-1,o||(l+1!==1e4?(a[f]=null,l++):o=this._createHashClone(a,f))}else o&&(o[f]=c)}o&&(r[s]=o)}destroy(){this.enabled=!1,this._managedResources.forEach(e=>{e.off("unload",this.removeResource,this)}),this._managedResources.length=0,this._managedResourceHashes.length=0,this._managedCollections.length=0,this._renderer=null}};de.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"gc",priority:0};de.defaultOptions={gcActive:!0,gcMaxUnusedTime:6e4,gcFrequency:3e4};let Hr=de;class xt{constructor(e){this._stackIndex=0,this._globalUniformDataStack=[],this._uniformsPool=[],this._activeUniforms=[],this._bindGroupPool=[],this._activeBindGroups=[],this._renderer=e}reset(){this._stackIndex=0;for(let e=0;e<this._activeUniforms.length;e++)this._uniformsPool.push(this._activeUniforms[e]);for(let e=0;e<this._activeBindGroups.length;e++)this._bindGroupPool.push(this._activeBindGroups[e]);this._activeUniforms.length=0,this._activeBindGroups.length=0}start(e){this.reset(),this.push(e)}bind({size:e,projectionMatrix:t,worldTransformMatrix:r,worldColor:s,offset:i}){const a=this._renderer.renderTarget.renderTarget,o=this._stackIndex?this._globalUniformDataStack[this._stackIndex-1]:{worldTransformMatrix:new M,worldColor:4294967295,offset:new Bt},l={projectionMatrix:t||this._renderer.renderTarget.projectionMatrix,resolution:e||a.size,worldTransformMatrix:r||o.worldTransformMatrix,worldColor:s||o.worldColor,offset:i||o.offset,bindGroup:null},d=this._uniformsPool.pop()||this._createUniforms();this._activeUniforms.push(d);const f=d.uniforms;f.uProjectionMatrix=l.projectionMatrix,f.uResolution=l.resolution,f.uWorldTransformMatrix.copyFrom(l.worldTransformMatrix),f.uWorldTransformMatrix.tx-=l.offset.x,f.uWorldTransformMatrix.ty-=l.offset.y,Sr(l.worldColor,f.uWorldColorAlpha,0),d.update();let c;this._renderer.renderPipes.uniformBatch?c=this._renderer.renderPipes.uniformBatch.getUniformBindGroup(d,!1):(c=this._bindGroupPool.pop()||new zt,this._activeBindGroups.push(c),c.setResource(d,0)),l.bindGroup=c,this._currentGlobalUniformData=l}push(e){this.bind(e),this._globalUniformDataStack[this._stackIndex++]=this._currentGlobalUniformData}pop(){this._currentGlobalUniformData=this._globalUniformDataStack[--this._stackIndex-1],this._renderer.type===ae.WEBGL&&this._currentGlobalUniformData.bindGroup.resources[0].update()}get bindGroup(){return this._currentGlobalUniformData.bindGroup}get globalUniformData(){return this._currentGlobalUniformData}get uniformGroup(){return this._currentGlobalUniformData.bindGroup.resources[0]}_createUniforms(){return new ie({uProjectionMatrix:{value:new M,type:"mat3x3<f32>"},uWorldTransformMatrix:{value:new M,type:"mat3x3<f32>"},uWorldColorAlpha:{value:new Float32Array(4),type:"vec4<f32>"},uResolution:{value:[0,0],type:"vec2<f32>"}},{isStatic:!0})}destroy(){this._renderer=null,this._globalUniformDataStack.length=0,this._uniformsPool.length=0,this._activeUniforms.length=0,this._bindGroupPool.length=0,this._activeBindGroups.length=0,this._currentGlobalUniformData=null}}xt.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"globalUniforms"};let Vr=1;class gt{constructor(){this._tasks=[],this._offset=0}init(){ge.system.add(this._update,this)}repeat(e,t,r=!0){const s=Vr++;let i=0;return r&&(this._offset+=1e3,i=this._offset),this._tasks.push({func:e,duration:t,start:performance.now(),offset:i,last:performance.now(),repeat:!0,id:s}),s}cancel(e){for(let t=0;t<this._tasks.length;t++)if(this._tasks[t].id===e){this._tasks.splice(t,1);return}}_update(){const e=performance.now();for(let t=0;t<this._tasks.length;t++){const r=this._tasks[t];if(e-r.offset-r.last>=r.duration){const s=e-r.start;r.func(s),r.last=e}}}destroy(){ge.system.remove(this._update,this),this._tasks.length=0}}gt.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"scheduler",priority:0};let Ee=!1;function Fr(n){if(!Ee){if(ne.get().getNavigator().userAgent.toLowerCase().indexOf("chrome")>-1){const e=[`%c  %c  %c  %c  %c PixiJS %c v${_e} (${n}) http://www.pixijs.com/

`,"background: #E72264; padding:5px 0;","background: #6CA2EA; padding:5px 0;","background: #B5D33D; padding:5px 0;","background: #FED23F; padding:5px 0;","color: #FFFFFF; background: #E72264; padding:5px 0;","color: #E72264; background: #FFFFFF; padding:5px 0;"];globalThis.console.log(...e)}else globalThis.console&&globalThis.console.log(`PixiJS ${_e} - ${n} - http://www.pixijs.com/`);Ee=!0}}class he{constructor(e){this._renderer=e}init(e){if(e.hello){let t=this._renderer.name;this._renderer.type===ae.WEBGL&&(t+=` ${this._renderer.context.webGLVersion}`),Fr(t)}}}he.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"hello",priority:-2};he.defaultOptions={hello:!1};const fe=class _t{constructor(e){this._renderer=e}init(e){e={..._t.defaultOptions,...e},this.maxUnusedTime=e.renderableGCMaxUnusedTime}get enabled(){return T("8.15.0","RenderableGCSystem.enabled is deprecated, please use the GCSystem.enabled instead."),this._renderer.gc.enabled}set enabled(e){T("8.15.0","RenderableGCSystem.enabled is deprecated, please use the GCSystem.enabled instead."),this._renderer.gc.enabled=e}addManagedHash(e,t){T("8.15.0","RenderableGCSystem.addManagedHash is deprecated, please use the GCSystem.addCollection instead."),this._renderer.gc.addCollection(e,t,"hash")}addManagedArray(e,t){T("8.15.0","RenderableGCSystem.addManagedArray is deprecated, please use the GCSystem.addCollection instead."),this._renderer.gc.addCollection(e,t,"array")}addRenderable(e){T("8.15.0","RenderableGCSystem.addRenderable is deprecated, please use the GCSystem instead."),this._renderer.gc.addResource(e,"renderable")}run(){T("8.15.0","RenderableGCSystem.run is deprecated, please use the GCSystem instead."),this._renderer.gc.run()}destroy(){this._renderer=null}};fe.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"renderableGC",priority:0};fe.defaultOptions={renderableGCActive:!0,renderableGCMaxUnusedTime:6e4,renderableGCFrequency:3e4};let Lr=fe;const pe=class H{get count(){return this._renderer.tick}get checkCount(){return this._checkCount}set checkCount(e){T("8.15.0","TextureGCSystem.run is deprecated, please use the GCSystem instead."),this._checkCount=e}get maxIdle(){return this._renderer.gc.maxUnusedTime/1e3*60}set maxIdle(e){T("8.15.0","TextureGCSystem.run is deprecated, please use the GCSystem instead."),this._renderer.gc.maxUnusedTime=e/60*1e3}get checkCountMax(){return Math.floor(this._renderer.gc._frequency/1e3)}set checkCountMax(e){T("8.15.0","TextureGCSystem.run is deprecated, please use the GCSystem instead.")}get active(){return this._renderer.gc.enabled}set active(e){T("8.15.0","TextureGCSystem.run is deprecated, please use the GCSystem instead."),this._renderer.gc.enabled=e}constructor(e){this._renderer=e,this._checkCount=0}init(e){e.textureGCActive!==H.defaultOptions.textureGCActive&&(this.active=e.textureGCActive),e.textureGCMaxIdle!==H.defaultOptions.textureGCMaxIdle&&(this.maxIdle=e.textureGCMaxIdle),e.textureGCCheckCountMax!==H.defaultOptions.textureGCCheckCountMax&&(this.checkCountMax=e.textureGCCheckCountMax)}run(){T("8.15.0","TextureGCSystem.run is deprecated, please use the GCSystem instead."),this._renderer.gc.run()}destroy(){this._renderer=null}};pe.extension={type:[u.WebGLSystem,u.WebGPUSystem],name:"textureGC"};pe.defaultOptions={textureGCActive:!0,textureGCAMaxIdle:null,textureGCMaxIdle:3600,textureGCCheckCountMax:600};let Or=pe;const bt=class vt{constructor(e={}){if(this.uid=ze("renderTarget"),this.colorTextures=[],this.dirtyId=0,this.isRoot=!1,this._size=new Float32Array(2),this._managedColorTextures=!1,e={...vt.defaultOptions,...e},this.stencil=e.stencil,this.depth=e.depth,this.isRoot=e.isRoot,typeof e.colorTextures=="number"){this._managedColorTextures=!0;for(let t=0;t<e.colorTextures;t++)this.colorTextures.push(new I({width:e.width,height:e.height,resolution:e.resolution,antialias:e.antialias}))}else{this.colorTextures=[...e.colorTextures.map(r=>r.source)];const t=this.colorTexture.source;this.resize(t.width,t.height,t._resolution)}this.colorTexture.source.on("resize",this.onSourceResize,this),(e.depthStencilTexture||this.stencil)&&(e.depthStencilTexture instanceof S||e.depthStencilTexture instanceof I?this.depthStencilTexture=e.depthStencilTexture.source:this.ensureDepthStencilTexture())}get size(){const e=this._size;return e[0]=this.pixelWidth,e[1]=this.pixelHeight,e}get width(){return this.colorTexture.source.width}get height(){return this.colorTexture.source.height}get pixelWidth(){return this.colorTexture.source.pixelWidth}get pixelHeight(){return this.colorTexture.source.pixelHeight}get resolution(){return this.colorTexture.source._resolution}get colorTexture(){return this.colorTextures[0]}onSourceResize(e){this.resize(e.width,e.height,e._resolution,!0)}ensureDepthStencilTexture(){this.depthStencilTexture||(this.depthStencilTexture=new I({width:this.width,height:this.height,resolution:this.resolution,format:"depth24plus-stencil8",autoGenerateMipmaps:!1,antialias:!1,mipLevelCount:1}))}resize(e,t,r=this.resolution,s=!1){this.dirtyId++,this.colorTextures.forEach((i,a)=>{s&&a===0||i.source.resize(e,t,r)}),this.depthStencilTexture&&this.depthStencilTexture.source.resize(e,t,r)}destroy(){this.colorTexture.source.off("resize",this.onSourceResize,this),this._managedColorTextures&&this.colorTextures.forEach(e=>{e.destroy()}),this.depthStencilTexture&&(this.depthStencilTexture.destroy(),delete this.depthStencilTexture)}};bt.defaultOptions={width:0,height:0,resolution:1,colorTextures:1,stencil:!1,depth:!1,antialias:!1,isRoot:!1};let ee=bt;const P=new Map;He.register(P);function yt(n,e){if(!P.has(n)){const t=new S({source:new X({resource:n,...e})}),r=()=>{P.get(n)===t&&P.delete(n)};t.once("destroy",r),t.source.once("destroy",r),P.set(n,t)}return P.get(n)}const me=class Tt{get autoDensity(){return this.texture.source.autoDensity}set autoDensity(e){this.texture.source.autoDensity=e}get resolution(){return this.texture.source._resolution}set resolution(e){this.texture.source.resize(this.texture.source.width,this.texture.source.height,e)}init(e){e={...Tt.defaultOptions,...e},e.view&&(T(Rt,"ViewSystem.view has been renamed to ViewSystem.canvas"),e.canvas=e.view),this.screen=new L(0,0,e.width,e.height),this.canvas=e.canvas||ne.get().createCanvas(),this.antialias=!!e.antialias,this.texture=yt(this.canvas,e),this.renderTarget=new ee({colorTextures:[this.texture],depth:!!e.depth,isRoot:!0}),this.texture.source.transparent=e.backgroundAlpha<1,this.resolution=e.resolution}resize(e,t,r){this.texture.source.resize(e,t,r),this.screen.width=this.texture.frame.width,this.screen.height=this.texture.frame.height}destroy(e=!1){(typeof e=="boolean"?e:!!(e!=null&&e.removeView))&&this.canvas.parentNode&&this.canvas.parentNode.removeChild(this.canvas),this.texture.destroy()}};me.extension={type:[u.WebGLSystem,u.WebGPUSystem,u.CanvasSystem],name:"view",priority:0};me.defaultOptions={width:800,height:600,autoDensity:!1,antialias:!1};let Wr=me;const Qr=[Ar,xt,he,Wr,dt,Hr,Or,pt,Ur,Pt,Lr,gt],Yr=[lt,tt,ot,at,rt,nt,st,it];function $r(n,e,t,r,s,i){const a=i?1:-1;return n.identity(),n.a=1/r*2,n.d=a*(1/s*2),n.tx=-1-e*n.a,n.ty=-a-t*n.d,n}function jr(n){const e=n.colorTexture.source.resource;return globalThis.HTMLCanvasElement&&e instanceof HTMLCanvasElement&&document.body.contains(e)}class Xr{constructor(e){this.rootViewPort=new L,this.viewport=new L,this.mipLevel=0,this.layer=0,this.onRenderTargetChange=new At("onRenderTargetChange"),this.projectionMatrix=new M,this.defaultClearColor=[0,0,0,0],this._renderSurfaceToRenderTargetHash=new Map,this._gpuRenderTargetHash=Object.create(null),this._renderTargetStack=[],this._renderer=e,e.gc.addCollection(this,"_gpuRenderTargetHash","hash")}finishRenderPass(){this.adaptor.finishRenderPass(this.renderTarget)}renderStart({target:e,clear:t,clearColor:r,frame:s,mipLevel:i,layer:a}){var o,l;this._renderTargetStack.length=0,this.push(e,t,r,s,i??0,a??0),this.rootViewPort.copyFrom(this.viewport),this.rootRenderTarget=this.renderTarget,this.renderingToScreen=jr(this.rootRenderTarget),(l=(o=this.adaptor).prerender)==null||l.call(o,this.rootRenderTarget)}postrender(){var e,t;(t=(e=this.adaptor).postrender)==null||t.call(e,this.rootRenderTarget)}bind(e,t=!0,r,s,i=0,a=0){const o=this.getRenderTarget(e),l=this.renderTarget!==o;this.renderTarget=o,this.renderSurface=e;const d=this.getGpuRenderTarget(o);(o.pixelWidth!==d.width||o.pixelHeight!==d.height)&&(this.adaptor.resizeGpuRenderTarget(o),d.width=o.pixelWidth,d.height=o.pixelHeight);const f=o.colorTexture,c=this.viewport,x=f.arrayLayerCount||1;if((a|0)!==a&&(a|=0),a<0||a>=x)throw new Error(`[RenderTargetSystem] layer ${a} is out of bounds (arrayLayerCount=${x}).`);this.mipLevel=i|0,this.layer=a|0;const g=Math.max(f.pixelWidth>>i,1),m=Math.max(f.pixelHeight>>i,1);if(!s&&e instanceof S&&(s=e.frame),s){const v=f._resolution,_=1<<Math.max(i|0,0),h=s.x*v+.5|0,C=s.y*v+.5|0,p=s.width*v+.5|0,y=s.height*v+.5|0;let b=Math.floor(h/_),B=Math.floor(C/_),W=Math.ceil(p/_),$=Math.ceil(y/_);b=Math.min(Math.max(b,0),g-1),B=Math.min(Math.max(B,0),m-1),W=Math.min(Math.max(W,1),g-b),$=Math.min(Math.max($,1),m-B),c.x=b,c.y=B,c.width=W,c.height=$}else c.x=0,c.y=0,c.width=g,c.height=m;return $r(this.projectionMatrix,0,0,c.width/f.resolution,c.height/f.resolution,!o.isRoot),this.adaptor.startRenderPass(o,t,r,c,i,a),l&&this.onRenderTargetChange.emit(o),o}clear(e,t=Q.ALL,r,s=this.mipLevel,i=this.layer){t&&(e&&(e=this.getRenderTarget(e)),this.adaptor.clear(e||this.renderTarget,t,r,this.viewport,s,i))}contextChange(){this._gpuRenderTargetHash=Object.create(null)}push(e,t=Q.ALL,r,s,i=0,a=0){const o=this.bind(e,t,r,s,i,a);return this._renderTargetStack.push({renderTarget:o,frame:s,mipLevel:i,layer:a}),o}pop(){this._renderTargetStack.pop();const e=this._renderTargetStack[this._renderTargetStack.length-1];this.bind(e.renderTarget,!1,null,e.frame,e.mipLevel,e.layer)}getRenderTarget(e){return e.isTexture&&(e=e.source),this._renderSurfaceToRenderTargetHash.get(e)??this._initRenderTarget(e)}copyToTexture(e,t,r,s,i){r.x<0&&(s.width+=r.x,i.x-=r.x,r.x=0),r.y<0&&(s.height+=r.y,i.y-=r.y,r.y=0);const{pixelWidth:a,pixelHeight:o}=e;return s.width=Math.min(s.width,a-r.x),s.height=Math.min(s.height,o-r.y),this.adaptor.copyToTexture(e,t,r,s,i)}ensureDepthStencil(){this.renderTarget.stencil||(this.renderTarget.stencil=!0,this.adaptor.startRenderPass(this.renderTarget,!1,null,this.viewport,0,this.layer))}destroy(){this._renderer=null,this._renderSurfaceToRenderTargetHash.forEach((e,t)=>{e!==t&&e.destroy()}),this._renderSurfaceToRenderTargetHash.clear(),this._gpuRenderTargetHash=Object.create(null)}_initRenderTarget(e){let t=null;return X.test(e)&&(e=yt(e).source),e instanceof ee?t=e:e instanceof I&&(t=new ee({colorTextures:[e]}),e.source instanceof X&&(t.isRoot=!0),e.once("destroy",()=>{t.destroy(),this._renderSurfaceToRenderTargetHash.delete(e);const r=this._gpuRenderTargetHash[t.uid];r&&(this._gpuRenderTargetHash[t.uid]=null,this.adaptor.destroyGpuRenderTarget(r))})),this._renderSurfaceToRenderTargetHash.set(e,t),t}getGpuRenderTarget(e){return this._gpuRenderTargetHash[e.uid]||(this._gpuRenderTargetHash[e.uid]=this.adaptor.initGpuRenderTarget(e))}resetState(){this.renderTarget=null,this.renderSurface=null}}export{rt as A,lt as B,it as C,Je as D,qr as G,Xr as R,k as S,dr as a,Qr as b,ur as c,Yr as d,Lt as e,ye as f,mr as g,cr as h,hr as i,br as j,gr as k,vr as l,Sr as m,tt as n,ot as o,at as p,_r as r};
