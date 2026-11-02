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

/* ---- Placements on a tile ---- */

typedef struct WaPlacement {
    /* For wa_adt_model_name or wa_adt_wmo_name. */
    uint32_t name;
    /* Shared by every tile that lists this placement. */
    uint32_t unique_id;
    /* East, up and south from the map's north-west corner. */
    float position[3];
    /* Degrees. */
    float rotation[3];
    float scale;
    uint32_t doodad_set;
    /* World bounds from the file. All zero for M2s. */
    float bounds_min[3];
    float bounds_max[3];
} WaPlacement;

WaStatus wa_adt_model_name(const WaAdt *adt, uint32_t index, const uint8_t **out, size_t *out_len);
WaStatus wa_adt_wmo_name(const WaAdt *adt, uint32_t index, const uint8_t **out, size_t *out_len);
void wa_adt_doodads(const WaAdt *adt, const WaPlacement **out, size_t *out_len);
void wa_adt_wmos(const WaAdt *adt, const WaPlacement **out, size_t *out_len);

/* One layer of liquid in one chunk, on the chunk's 9x9 grid. */
typedef struct WaLiquidLayer {
    uint32_t chunk;       /* Y index * 16 + X index */
    uint32_t liquid_type; /* row in LiquidType.dbc */
    uint64_t cells;       /* bit row * 8 + column is set when that cell has liquid */
    const float *heights; /* 81 world heights, row by row */
    const uint8_t *depths; /* 81 depths from 0 to 255, row by row */
} WaLiquidLayer;

uint32_t wa_adt_liquid_count(const WaAdt *adt);
WaStatus wa_adt_liquid(const WaAdt *adt, uint32_t index, WaLiquidLayer *out);

/* ---- Models ---- */

#define WA_NO_TEXTURE 0xFFFFFFFFu
#define WA_GPU_RGBA8 0
#define WA_GPU_DXT1 1
#define WA_GPU_DXT3 3
#define WA_GPU_DXT5 5

typedef struct WaBatch {
    uint32_t index_start;
    uint32_t index_count;
    uint32_t material;
} WaBatch;

/* Pointers are owned by the handle the mesh came from. Positions and normals
   are in the file's axes, Z up. */
typedef struct WaMesh {
    uint32_t vertex_count;
    const float *positions;
    const float *normals;
    const float *tex_coords;
    /* Red, green, blue, alpha. NULL when there are none. */
    const uint8_t *colors;
    uint32_t index_count;
    const uint32_t *indices;
    uint32_t batch_count;
    const WaBatch *batches;
    /* Triangles to collide with. Empty for M2s, which have wa_m2_collision. */
    uint32_t collision_index_count;
    const uint32_t *collision_indices;
} WaMesh;

typedef struct WaM2Material {
    /* For wa_m2_texture_name, or WA_NO_TEXTURE. */
    uint32_t texture;
    uint32_t blend_mode;
    uint32_t flags;
    uint32_t texture_flags;
} WaM2Material;

typedef struct WaM2 WaM2;

/* An M2 and its .skin file. Returns NULL on failure. */
WaM2 *wa_m2_open(const uint8_t *m2, size_t m2_len, const uint8_t *skin, size_t skin_len);
void wa_m2_free(WaM2 *model);
void wa_m2_mesh(const WaM2 *model, WaMesh *out);
float wa_m2_bounding_radius(const WaM2 *model);
/* The model's collision mesh, which has its own vertices: 3 floats each, Z up.
   Owned by model, and empty for models that have none. */
void wa_m2_collision(const WaM2 *model, const float **positions, uint32_t *position_count, const uint32_t **indices, uint32_t *index_count);
/* One material per batch. */
WaStatus wa_m2_material(const WaM2 *model, uint32_t index, WaM2Material *out);
WaStatus wa_m2_texture_name(const WaM2 *model, uint32_t index, const uint8_t **out, size_t *out_len);

typedef struct WaWmoMaterial {
    uint32_t flags;
    uint32_t blend_mode;
} WaWmoMaterial;

typedef struct WaWmoDoodadSet {
    uint32_t start;
    uint32_t count;
} WaWmoDoodadSet;

typedef struct WaWmoDoodad {
    uint32_t name;
    float position[3];
    /* X, Y, Z, W. */
    float rotation[4];
    float scale;
} WaWmoDoodad;

typedef struct WaWmo WaWmo;
typedef struct WaWmoGroup WaWmoGroup;

/* A WMO root file. Returns NULL on failure. */
WaWmo *wa_wmo_open(const uint8_t *data, size_t len);
void wa_wmo_free(WaWmo *wmo);
uint32_t wa_wmo_group_count(const WaWmo *wmo);
/* Writes 6 floats: the minimum corner, then the maximum, in the WMO's own space. */
void wa_wmo_bounds(const WaWmo *wmo, float *out);
uint32_t wa_wmo_material_count(const WaWmo *wmo);
WaStatus wa_wmo_material(const WaWmo *wmo, uint32_t index, WaWmoMaterial *out, const uint8_t **texture, size_t *texture_len);
uint32_t wa_wmo_doodad_set_count(const WaWmo *wmo);
WaStatus wa_wmo_doodad_set(const WaWmo *wmo, uint32_t index, WaWmoDoodadSet *out);
uint32_t wa_wmo_doodad_count(const WaWmo *wmo);
WaStatus wa_wmo_doodad(const WaWmo *wmo, uint32_t index, WaWmoDoodad *out);
WaStatus wa_wmo_doodad_name(const WaWmo *wmo, uint32_t index, const uint8_t **out, size_t *out_len);

/* A WMO group file, named <root>_NNN.wmo. Returns NULL on failure. */
WaWmoGroup *wa_wmo_group_open(const uint8_t *data, size_t len);
void wa_wmo_group_free(WaWmoGroup *group);
void wa_wmo_group_mesh(const WaWmoGroup *group, WaMesh *out);
uint32_t wa_wmo_group_flags(const WaWmoGroup *group);
uint32_t wa_wmo_flags(const WaWmo *wmo);

/* A group's liquid surface, on a grid in the WMO's own space. */
typedef struct WaWmoLiquid {
    uint32_t vertices_x;
    uint32_t vertices_y;
    float corner[3];        /* X, Y, Z */
    const float *heights;   /* vertices_x * vertices_y heights, row by row */
    const uint8_t *cells;   /* one byte per cell, row by row: nonzero when it has liquid */
    uint32_t liquid_type;   /* row in LiquidType.dbc */
} WaWmoLiquid;

/* root_flags comes from wa_wmo_flags. WA_NOT_FOUND when the group has no liquid. */
WaStatus wa_wmo_group_liquid(const WaWmoGroup *group, uint32_t root_flags, WaWmoLiquid *out);

typedef struct WaGpuTexture {
    WaBuffer pixels;
    /* WA_GPU_RGBA8 holds one level; the others hold a chain down to 1 pixel. */
    uint32_t format;
    uint32_t width;
    uint32_t height;
    uint32_t mip_count;
} WaGpuTexture;

/* The file's own compressed mipmaps when the GPU can use them, or else the first
   level as RGBA8. Free pixels with wa_buffer_free. */
WaStatus wa_blp_to_gpu(const uint8_t *data, size_t len, WaGpuTexture *out);

#endif

#endif
