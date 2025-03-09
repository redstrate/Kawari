#include <cstdint>
#include <cstdlib>
#include "blowfish.h"
#include "wrapper.h"

// the contents of blowfish.cpp and it's headers are directly from sapphire server (which I believe is ripped from the game but with prep methods)
// https://github.com/SapphireServer/Sapphire/blob/master/src/common/Crypt/blowfish.cpp
// TODO: eventually get the lobby to work with the blowfish or blowfish_rs crate.

BYTE *blowfish_encode(BYTE *key, uint32_t keybytes, BYTE *pInput, DWORD lSize)
{
	BlowFish blowfish;
	blowfish.initialize(key, keybytes);

	BYTE *pOutput = new BYTE[lSize];
	blowfish.Encode(pInput, pOutput, lSize);

	return pOutput;
}

BYTE *blowfish_decode(BYTE *key, uint32_t keybytes, BYTE *pInput, DWORD lSize)
{
	BlowFish blowfish;
	blowfish.initialize(key, keybytes);

	BYTE *pOutput = new BYTE[lSize];
	blowfish.Decode(pInput, pOutput, lSize);

	return pOutput;
}
