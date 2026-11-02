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
    /* Each layer's GroundEffectTexture row. */
    uint32_t effect_ids[4];
    /* Alpha for layers 1 to 3: 3 maps of 64x64 bytes, row by row. */
    const uint8_t *alpha;
    /* 145 colors of 4 bytes as stored: blue, green, red, alpha. 127 is neutral. */
    const uint8_t *vertex_colors;
    /* One bit per cell of the chunk's 8 by 8, row by row: no ground effect doodads there. */
    uint64_t no_effect_doodad;
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
    /* M2 texture type, even without a name: 0 is the named file, and others are chosen at
       runtime, such as 1 for a character's body and 11 to 13 for a creature's skins. */
    uint32_t texture_type;
    /* The batch's geoset id, for models that show only some of theirs. */
    uint32_t geoset;
    /* The batch as stored, for working out how it combines its textures. */
    uint32_t shader_id;
    /* How many textures it uses, from 1. */
    uint32_t texture_count;
    uint32_t material_index;
    uint32_t material_layer;
    /* M2 batch flags: 0x10 means its texture doesn't animate. */
    uint32_t batch_flags;
    /* For wa_m2_texture_name and wa_m2_texture_type, even when the name is empty, or WA_NO_TEXTURE. */
    uint32_t textures[2];
    /* Which UV set each texture uses: 0, 1, or -1 for a sphere map. */
    int32_t tex_coords[2];
    /* Indexes for wa_m2_effects_track, or -1. */
    int32_t texture_transforms[2];
    int32_t color;
    int32_t texture_weight;
} WaM2Material;

typedef struct WaM2Bone {
    /* A bone index, or -1. */
    int32_t parent;
    int32_t key_bone;
    uint32_t flags;
    float pivot[3];
} WaM2Bone;

typedef struct WaM2Sequence {
    /* An AnimationData row. */
    uint32_t id;
    uint32_t variation;
    uint32_t duration_ms;
    /* Yards per second that its feet cover. */
    float movement_speed;
    /* 0x20: its keys are in the M2, and otherwise in its .anim file. 0x40: it has no keys of
       its own and uses the alias_next sequence's. */
    uint32_t flags;
    /* How often this variation plays, out of 32767 across a sequence's variations. */
    int32_t frequency;
    /* The sequence index of the next variation of this id, or -1. */
    int32_t next_variation;
    uint32_t alias_next;
    uint32_t blend_time_ms;
    float bounds_min[3];
    float bounds_max[3];
    float bounds_radius;
} WaM2Sequence;

typedef struct WaM2Attachment {
    /* Where it goes: 5 and 6 are the shoulders, and 11 the helmet. */
    uint32_t id;
    uint32_t bone;
    float position[3];
} WaM2Attachment;

#define WA_M2_TRACK_TRANSLATION 0
#define WA_M2_TRACK_ROTATION 1
#define WA_M2_TRACK_SCALE 2

/* One bone's keys for one sequence. Pointers are owned by the clip. */
typedef struct WaM2Track {
    /* 0 steps between keys, 1 is linear, and 2 and 3 are curves whose tangents aren't given. */
    uint32_t interpolation;
    /* A wa_m2_global_sequences index that drives this track instead of the sequence, or -1. */
    int32_t global_sequence;
    uint32_t key_count;
    /* In milliseconds. */
    const uint32_t *times;
    /* 3 floats per key, or 4 for rotations: X, Y, Z, W. Rotations are decompressed. */
    const float *values;
} WaM2Track;

typedef struct WaM2Clip WaM2Clip;

/* Something a model asks for at moments in its sequences, such as a footstep. */
typedef struct WaM2Event {
    /* 4 characters, usually $ and 3 more, such as $FD1. */
    uint8_t identifier[4];
    uint32_t data;
    uint32_t bone;
    float position[3];
} WaM2Event;

#define WA_M2_EFFECT_COLOR 0
#define WA_M2_EFFECT_WEIGHT 1
#define WA_M2_EFFECT_TRANSFORM 2

/* Tracks that change how a batch looks, as read from the first sequence and from global
   sequences. Tracks whose keys are outside the M2 come back empty. */
typedef struct WaM2Effects WaM2Effects;

typedef struct WaM2 WaM2;

/* An M2 and its .skin file. Returns NULL on failure. */
WaM2 *wa_m2_open(const uint8_t *m2, size_t m2_len, const uint8_t *skin, size_t skin_len);

/* A particle emitter's settings. Animated ones are their first key, of the first sequence that
   has any. Colors are 0 to 255, three keys over a particle's life at 0, mid and 1. */
typedef struct WaM2Emitter {
    uint32_t flags;
    /* In the model's axes and space, not its bone's. */
    float position[3];
    int32_t bone;
    /* For wa_m2_texture_name and wa_m2_texture_type. */
    uint32_t texture;
    uint32_t blend;
    uint32_t emitter_type;
    uint32_t particle_type;
    uint32_t rows;
    uint32_t columns;
    float speed;
    float speed_variation;
    float vertical_range;
    float horizontal_range;
    float gravity;
    float lifespan;
    float lifespan_variation;
    float rate;
    float rate_variation;
    float area_length;
    float area_width;
    float mid;
    float colors[9];
    float alphas[3];
    float sizes[3];
    /* 0 when the emitter starts switched off. */
    uint32_t enabled;
    /* Which tile of the texture shows over a particle's life, as up to 4 keys of a time from 0
       to 1 and a tile. Between keys it runs from one to the next. 0 keys means no animation. */
    uint32_t cell_count;
    float cell_times[4];
    float cell_values[4];
} WaM2Emitter;

uint32_t wa_m2_emitter_count(const WaM2 *model);
WaStatus wa_m2_emitter(const WaM2 *model, uint32_t index, WaM2Emitter *out);
void wa_m2_free(WaM2 *model);
void wa_m2_mesh(const WaM2 *model, WaMesh *out);
float wa_m2_bounding_radius(const WaM2 *model);
/* The model's collision mesh, which has its own vertices: 3 floats each, Z up.
   Owned by model, and empty for models that have none. */
void wa_m2_collision(const WaM2 *model, const float **positions, uint32_t *position_count, const uint32_t **indices, uint32_t *index_count);
/* One material per batch. */
WaStatus wa_m2_material(const WaM2 *model, uint32_t index, WaM2Material *out);
WaStatus wa_m2_texture_name(const WaM2 *model, uint32_t index, const uint8_t **out, size_t *out_len);
/* 2 floats per vertex, owned by model. */
const float *wa_m2_tex_coords2(const WaM2 *model);
/* An M2 texture type and its flags, which replaceable textures have too. */
WaStatus wa_m2_texture_type(const WaM2 *model, uint32_t index, uint32_t *texture_type, uint32_t *flags);
/* How a model's batches combine their textures, when its flag 0x8 says it has a table: each
   batch's shader_id indexes it. Returns 0 without one, and otherwise 1. Owned by model. */
uint32_t wa_m2_combiner_overrides(const WaM2 *model, const uint16_t **out, uint32_t *count);
/* The length of the first sequence in milliseconds, which effect tracks not on a global
   sequence loop over. 0 for a model without sequences. */
uint32_t wa_m2_first_sequence_ms(const WaM2 *model);
/* Never fails, because a track that can't be read comes back empty. */
WaM2Effects *wa_m2_effects_open(const WaM2 *model);
void wa_m2_effects_free(WaM2Effects *effects);
uint32_t wa_m2_effects_count(const WaM2Effects *effects, uint32_t family);
/* family is a WA_M2_EFFECT_ value. kind is, for colors, 0 for RGB floats and 1 for an alpha
   float; for weights, 0 for a float; and for transforms, 0 for translation, 1 for a rotation
   of X, Y, Z, W, and 2 for scale. Alphas and weights run from 0 to 1. */
WaStatus wa_m2_effects_track(const WaM2Effects *effects, uint32_t family, uint32_t index, uint32_t kind, WaM2Track *out);
/* 2 pointers to 4 bytes per vertex, owned by model. Indices are into the model's bones, not
   through a lookup table. Weights add up to 255. */
void wa_m2_vertex_bones(const WaM2 *model, const uint8_t **weights, const uint8_t **indices);
uint32_t wa_m2_bone_count(const WaM2 *model);
WaStatus wa_m2_bone(const WaM2 *model, uint32_t index, WaM2Bone *out);
uint32_t wa_m2_sequence_count(const WaM2 *model);
WaStatus wa_m2_sequence(const WaM2 *model, uint32_t index, WaM2Sequence *out);
/* Each global sequence's length in milliseconds. Owned by model. */
void wa_m2_global_sequences(const WaM2 *model, const uint32_t **out, uint32_t *count);
uint32_t wa_m2_attachment_count(const WaM2 *model);
WaStatus wa_m2_attachment(const WaM2 *model, uint32_t index, WaM2Attachment *out);
/* Reads a sequence's keys. A sequence stored in the M2 passes NULL for anim. Otherwise anim is
   the file <model><id:04>-<variation:02>.anim. An alias has no keys, so pass its target.
   Returns NULL on failure. */
WaM2Clip *wa_m2_clip_open(const WaM2 *model, uint32_t sequence, const uint8_t *anim, size_t anim_len);
void wa_m2_clip_free(WaM2Clip *clip);
uint32_t wa_m2_event_count(const WaM2 *model);
WaStatus wa_m2_event(const WaM2 *model, uint32_t index, WaM2Event *out);
/* When an event fires in the clip's sequence, in milliseconds. Owned by clip. */
WaStatus wa_m2_clip_event_times(const WaM2Clip *clip, uint32_t event, const uint32_t **out, uint32_t *count);
/* kind is a WA_M2_TRACK_ value. */
WaStatus wa_m2_clip_track(const WaM2Clip *clip, uint32_t bone, uint32_t kind, WaM2Track *out);

typedef struct WaWmoMaterial {
    uint32_t flags;
    uint32_t blend_mode;
    uint32_t shader;
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
    /* Its baked light, as stored: 0xAARRGGBB. */
    uint32_t color;
} WaWmoDoodad;

/* One portal, as corners in wa_wmo_portal_vertices and a plane. */
typedef struct WaWmoPortal {
    uint32_t start;
    uint32_t count;
    float normal[3];
    float offset;
} WaWmoPortal;

/* A group's link through a portal. */
typedef struct WaWmoPortalRef {
    uint32_t portal;
    /* The group on the far side. */
    uint32_t group;
    /* The side of the portal's plane it's seen from: 1, -1, or 0 for both. */
    int32_t side;
} WaWmoPortalRef;

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
/* The doodads standing in this group, for wa_wmo_doodad. Owned by group. */
void wa_wmo_group_doodad_refs(const WaWmoGroup *group, const uint16_t **out, uint32_t *count);
/* 3 floats per corner, in the file's axes. Owned by wmo. */
void wa_wmo_portal_vertices(const WaWmo *wmo, const float **out, uint32_t *count);
uint32_t wa_wmo_portal_count(const WaWmo *wmo);
WaStatus wa_wmo_portal(const WaWmo *wmo, uint32_t index, WaWmoPortal *out);
uint32_t wa_wmo_portal_ref_count(const WaWmo *wmo);
WaStatus wa_wmo_portal_ref(const WaWmo *wmo, uint32_t index, WaWmoPortalRef *out);
/* The group's box in the file's axes: smallest corner, then largest. 6 floats. */
void wa_wmo_group_bounds(const WaWmoGroup *group, float *out);
/* The group's range of wa_wmo_portal_ref. */
void wa_wmo_group_portal_refs(const WaWmoGroup *group, uint32_t *start, uint32_t *count);
/* How many of the mesh's batches, from the first, are transition batches. */
uint32_t wa_wmo_group_transition_batch_count(const WaWmoGroup *group);
/* The WMO's ambient color, as stored: 0xAARRGGBB. */
uint32_t wa_wmo_ambient_color(const WaWmo *wmo);
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
