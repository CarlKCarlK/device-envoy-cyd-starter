# Paint Book assets

The application embeds four uncompressed 24-bit 320x240 TGA pages:

- `paint-dog-walk.tga`
- `paint-garden.tga`
- `paint-ocean.tga`
- `paint-space.tga`

Their high-resolution editable sources use corresponding PNG names. Device
Envoy converts the TGA pixels to RGB565 at compile time.

`dog_walk.png` is the original dog-walk illustration. The page-ready
`dog-walk-page-source.png` was made from it with OpenAI's built-in image editing
tool using this prompt: preserve the original illustration exactly and add only
a folded top-right page corner with a right-pointing arrow; add no text, logo,
signature, or watermark.

The other three sources were generated for this repository with OpenAI's
built-in image generation tool. The shared prompt requested a polished
children's paint-book illustration in a 4:3 landscape composition, with a warm
cream central page, large high-contrast outlines, tactile gouache and watercolor
borders, six generous pools of vivid paint along the left and bottom edges, and
a folded top-right page corner containing a right arrow. It prohibited words,
letters, numbers, logos, watermarks, and small details. Their subject prompts
were:

- Garden: flowers, leaves, a butterfly, and a smiling sun.
- Ocean: a friendly fish, small octopus, shells, seaweed, bubbles, and waves.
- Space: a friendly rocket, ringed planet, crescent moon, stars, and a small
  smiling alien.
