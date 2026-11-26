# Vendored crates

## `miniquad` 0.4.11 (patched)

An unmodified copy of `miniquad` 0.4.11 from crates.io (upstream git
`4f13d4a70caf6a8cbf0187a95461658e7812b566`), wired in through `[patch.crates-io]` in
`Cargo.toml`, plus the small patch below (every changed line is marked `[shiftella patch]`).

### Why

Black boxes instead of text (mostly the Russian UI, intermittently ~50 % of runs on the main
menu). When a macroquad font atlas runs out of room it doubles: `Atlas::texture()` deletes the
old GL texture and creates a bigger one. miniquad's GL backend keeps a cache of which texture
is bound to each unit, but `delete_texture` did not update it (the `clear_texture_bindings()`
call there is commented out upstream). OpenGL unbinds a deleted texture and usually hands the
same name to the very next `glGenTextures`, so the new atlas texture got the old name, the
cache believed it was already bound and skipped `glBindTexture`, and the upload went to texture
0. The new atlas texture stayed empty/incomplete and every glyph drawn from it was black, every
frame, until the atlas grew again.

### What the patch does

1. `delete_texture` no longer frees a GL texture immediately: it is queued and freed in
   `commit_frame` (end of the frame). Draw calls batched earlier in the same frame — e.g. glyph
   quads of a font whose atlas regrows mid-batch — keep sampling a live texture, and a deleted
   name can't be reused inside the frame.
2. When the queued textures are freed, they are also dropped from the binding cache
   (`GlCache::forget_texture`), so a reused GL name is always really bound.
3. `#[allow(dead_code)]` on an Android-only function, to keep our builds warning-free.

### Updating

When bumping macroquad/miniquad, check whether upstream fixed the binding cache in
`delete_texture`; if so drop `vendor/miniquad` and the `[patch.crates-io]` entry, otherwise
re-copy the new version from `~/.cargo/registry/src/*/miniquad-*` and re-apply this diff.

```diff
--- a/src/graphics/gl/cache.rs
+++ b/src/graphics/gl/cache.rs
@@ -108,6 +108,22 @@
         self.bind_texture(slot_index, self.stored_target, self.stored_texture);
     }
 
+    /// [shiftella patch] Forget a deleted texture: GL has already unbound it everywhere.
+    pub fn forget_texture(&mut self, texture: GLuint) {
+        for cached in self.textures.iter_mut() {
+            if cached.texture == texture {
+                *cached = CachedTexture {
+                    target: 0,
+                    texture: 0,
+                };
+            }
+        }
+        if self.stored_texture == texture {
+            self.stored_target = 0;
+            self.stored_texture = 0;
+        }
+    }
+
     pub fn clear_buffer_bindings(&mut self) {
         self.bind_buffer(GL_ARRAY_BUFFER, 0, None);
         self.vertex_buffer = 0;
--- a/src/graphics/gl.rs
+++ b/src/graphics/gl.rs
@@ -519,6 +519,8 @@
     default_framebuffer: GLuint,
     pub(crate) cache: GlCache,
     pub(crate) info: ContextInfo,
+    /// [shiftella patch] GL textures released by `delete_texture`, freed in `commit_frame`.
+    deleted_textures: Vec<GLuint>,
 }
 
 impl Default for GlContext {
@@ -548,6 +550,7 @@
                 buffers: ResourceManager::default(),
                 textures: Textures(vec![]),
                 info,
+                deleted_textures: Vec::new(),
                 cache: GlCache {
                     stored_index_buffer: 0,
                     stored_index_type: None,
@@ -915,9 +918,9 @@
 
         let t = self.textures.get(texture);
         match &t.raw {
-            TextureOrRenderbuffer::Texture(raw) => unsafe {
-                glDeleteTextures(1, raw as *const _);
-            },
+            // [shiftella patch] Freed at the end of the frame: draw calls batched earlier in
+            // this frame may still sample it (e.g. a font atlas that regrew mid-batch).
+            TextureOrRenderbuffer::Texture(raw) => self.deleted_textures.push(*raw),
             TextureOrRenderbuffer::Renderbuffer(raw) => unsafe {
                 glDeleteRenderbuffers(1, raw as *const _);
             },
@@ -1720,6 +1723,14 @@
     fn commit_frame(&mut self) {
         self.cache.clear_buffer_bindings();
         self.cache.clear_texture_bindings();
+        // [shiftella patch] GL unbinds a deleted texture and may hand its name to the next
+        // glGenTextures: drop it from the binding cache too, or that new texture is never
+        // really bound and its upload lands in texture 0 (black boxes after a font atlas
+        // regrows).
+        for raw in std::mem::take(&mut self.deleted_textures) {
+            unsafe { glDeleteTextures(1, &raw as *const _) };
+            self.cache.forget_texture(raw);
+        }
     }
 
     fn draw(&self, base_element: i32, num_elements: i32, num_instances: i32) {
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -99,6 +99,8 @@
 }
 /// This for now is Android specific since the process can continue running but the display
 /// is restarted. We support reinitializing the display.
+// [shiftella patch] silence the upstream dead-code warning on desktop builds
+#[cfg_attr(not(target_os = "android"), allow(dead_code))]
 fn set_or_replace_display(display: native::NativeDisplayData) {
     if let Some(m) = NATIVE_DISPLAY.get() {
         // Replace existing display
```
