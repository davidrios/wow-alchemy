#ifndef WOW_ALCHEMY_H
#define WOW_ALCHEMY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum WaStatus {
    WA_OK = 0,
    WA_NOT_FOUND = 1,
    WA_ERROR = -1
} WaStatus;

typedef struct WaMpqChain WaMpqChain;
typedef struct WaDbc WaDbc;
typedef struct WaAdt WaAdt;

/* Bytes owned by the caller. Free with wa_buffer_free. */
typedef struct WaBuffer {
    uint8_t *ptr;
    size_t len;
} WaBuffer;

/* An RGBA8 image. Free pixels with wa_buffer_free. */
typedef struct WaImage {
    WaBuffer pixels;
    uint32_t width;
    uint32_t height;
    /* Mipmaps in the source texture, for use as mip_level. */
    uint32_t mip_count;
} WaImage;

/* The last failure on this thread as UTF-8, or NULL. Valid until the next
   wow-alchemy call on this thread. */
const char *wa_last_error(void);

void wa_buffer_free(WaBuffer buffer);

/* MPQ archives layered by priority: the higher priority wins and patches apply
   automatically. Paths and file names are NUL-terminated UTF-8. */
WaMpqChain *wa_mpq_chain_new(void);
void wa_mpq_chain_free(WaMpqChain *chain);
WaStatus wa_mpq_chain_add(WaMpqChain *chain, const char *path, int32_t priority);
int32_t wa_mpq_chain_contains(WaMpqChain *chain, const char *file_name);
/* WA_NOT_FOUND if no archive has the file. */
WaStatus wa_mpq_chain_read(WaMpqChain *chain, const char *file_name, WaBuffer *out);
/* All known file names as UTF-8, one per line. */
WaStatus wa_mpq_chain_list(WaMpqChain *chain, WaBuffer *out);

/* Decodes one mipmap level of a BLP texture to RGBA8. */
WaStatus wa_blp_decode(const uint8_t *data, size_t len, uint32_t mip_level, WaImage *out);

/* The size of a BLP texture's first mipmap. */
WaStatus wa_blp_size(const uint8_t *data, size_t len, uint32_t *width, uint32_t *height);
/* Converts a BLP texture to a BC1 mipmap chain for a GPU, dropping alpha. size is
   the width and height of the first level, a power of two, and the levels run
   down to 1 pixel. Square DXT textures at least that wide convert without loss. */
WaStatus wa_blp_to_bc1(const uint8_t *data, size_t len, uint32_t size, WaBuffer *out);

/* A DBC table. The data is copied, so it can be freed after this call. Returns
   NULL on failure. */
WaDbc *wa_dbc_open(const uint8_t *data, size_t len);
void wa_dbc_free(WaDbc *dbc);
void wa_dbc_info(const WaDbc *dbc, uint32_t *record_count, uint32_t *field_count, uint32_t *record_size);
/* Reads a 32-bit field. Reinterpret the bits for signed and float fields.
   WA_NOT_FOUND if the record or field is out of range. */
WaStatus wa_dbc_get_u32(const WaDbc *dbc, uint32_t record, uint32_t field, uint32_t *out);
/* Reads a string field. The UTF-8 bytes are owned by dbc and aren't
   NUL-terminated. WA_NOT_FOUND if the record, field or string is out of range. */
WaStatus wa_dbc_get_string(const WaDbc *dbc, uint32_t record, uint32_t field, const uint8_t **out, size_t *out_len);

/* The map's tiles: tiles must hold 4096 bytes, set to 1 where tile[y * 64 + x]
   exists. flags receives the MPHD flags, where 0x4 means big alpha maps. */
WaStatus wa_wdt_read(const uint8_t *data, size_t len, uint8_t *tiles, uint32_t *flags);

/* One map chunk of a tile. Pointers are owned by the WaAdt. */
typedef struct WaTerrainChunk {
    uint32_t ix;
    uint32_t iy;
    uint32_t flags;
    uint32_t area_id;
    /* 4x4 mask of holes in the terrain. */
    uint32_t holes;
    /* World position of the chunk's corner: north, west, base height. */
    float position[3];
    /* 145 heights relative to position[2], in alternating rows of 9 and 8. */
    const float *heights;
    /* 145 normals of 3 signed bytes, as stored in the file. */
    const int8_t *normals;
    uint32_t layer_count;
    /* Indexes for wa_adt_texture_name. */
    uint32_t texture_ids[4];
    /* MCLY flags of each layer. 0x400 means the texture is a reflection map, not a surface. */
    uint32_t layer_flags[4];
    /* Alpha for layers 1 to 3: 3 maps of 64x64 bytes, row by row. */
    const uint8_t *alpha;
    /* 145 colors of 4 bytes as stored: blue, green, red, alpha. 127 is neutral. */
    const uint8_t *vertex_colors;
} WaTerrainChunk;

/* An ADT terrain tile, fully parsed by this call. Set big_alpha to the WDT's
   big alpha flag. Returns NULL on failure. */
WaAdt *wa_adt_open(const uint8_t *data, size_t len, int32_t big_alpha);
void wa_adt_free(WaAdt *adt);
uint32_t wa_adt_texture_count(const WaAdt *adt);
/* The UTF-8 bytes are owned by adt and aren't NUL-terminated. */
WaStatus wa_adt_texture_name(const WaAdt *adt, uint32_t index, const uint8_t **out, size_t *out_len);
/* Chunks 0 to 255, ordered by iy then ix. */
WaStatus wa_adt_chunk(const WaAdt *adt, uint32_t index, WaTerrainChunk *out);

#endif

#endif
