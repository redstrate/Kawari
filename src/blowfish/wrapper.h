#include <cstdint>
extern "C"
{

	typedef void *BlowfishHandle;

#define DWORD uint32_t
#define WORD unsigned short
#define BYTE uint8_t

	struct ExpandedKey
	{
		DWORD *PArray;
		DWORD (*SBoxes)
		[256];
	};

	BYTE *blowfish_encode(BYTE *key, uint32_t keybytes, BYTE *pInput, DWORD lSize);
	BYTE *blowfish_decode(BYTE *key, uint32_t keybytes, BYTE *pInput, DWORD lSize);
	//__declspec(dllexport) void destroy(BlowFish hanle);
}
