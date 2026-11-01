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

#ifdef __cplusplus
}
#endif

#endif
