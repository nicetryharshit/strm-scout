# Reverse-engineered STRM v2 layout

All integers are little-endian.

| Offset | Type | Meaning |
|---:|---|---|
| 0 | 4 bytes | `STRM` magic |
| 4 | `u32` | File version, currently `2` |
| 8 | `u32` | Texture width |
| 12 | `u32` | Texture height |
| 16 | `u32` | Frame count |
| 20 | `u32` | BC7 encoder/quality enum |
| 24 | `u32` | Alpha type enum |
| 28 | `u32` | Frame compression enum, `2` is raw LZ4 block |
| 32 | `u32` | Frames per second |
| 36 | `u32` | Total compressed frame bytes |
| 40 | `u32` | Reserved |
| 44 | repeated pair of `u32` | Absolute frame offset and compressed size |

Each frame is a raw LZ4 block. Its decoded size is:

```text
ceil(width / 4) × ceil(height / 4) × 16
```

The result is a BC7 block texture, decoded to BGRA32 by `texture2ddecoder` and channel-swapped to RGBA8 for display and PNG export.

After the final frame is an eight-byte user-data header:

| Offset | Type | Meaning |
|---:|---|---|
| 0 | `u32` | User blob size |
| 4 | `u32` | Reserved |
| 8 | bytes | `StrmConfig` blob |

## StrmConfig v1

The blob contains the config version, frame-rate override, next strong ID, range ID list, and range list. Strong IDs are stored as two `u32` values. Strings use the .NET `BinaryWriter` UTF-8 string representation with a 7-bit encoded byte length.

Each range contains:

1. Range serialization version
2. Strong ID
3. Name
4. Beginning frame
5. Ending frame
6. End action
7. End-action target strong ID

Negative beginning and ending frames represent the entire stream.
