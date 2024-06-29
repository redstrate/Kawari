struct PatchEntry {
    url: String,
    version: String,
    hash_block_size: i64,
    length: i64,
    hashes: Vec<String>
}

struct PatchList {
    // FIXME: this is most likely auto-generated, not set?
    id: String,
    patches: Vec<PatchEntry>
}

impl PatchList {
    fn to_string(&self) -> String {
        let mut str = String::new();

        // header
        str.push_str("--");
        str.push_str(&self.id);
        str.push_str("\r\n");
        str.push_str("Content-Type: application/octet-stream\r\n");
        str.push_str("Content-Location: ffxivpatch/4e9a232b/metainfo/2023.07.26.0000.0000.http\r\n"); // TODO: hardcoded
        str.push_str("X-Patch-Length: 1664916486\r\n");
        str.push_str("\r\n");

        for patch in &self.patches {
            // length
            str.push_str(&patch.length.to_string());
            str.push_str("\t");

            // TODO: unknown value, but i *suspect* is the size of the game on disk once this patch is applied.
            // which would make sense for the launcher to check for
            str.push_str("44145529682");
            str.push_str("\t");

            // TODO: totally unknown
            str.push_str("71");
            str.push_str("\t");

            // TODO: unknown too
            str.push_str("11");
            str.push_str("\t");

            // version (e.g. 2023.09.15.0000.0000)
            str.push_str(&patch.version);
            str.push_str("\t");

            // hash type
            // TODO: does this need to be configurable?
            str.push_str("sha1");
            str.push_str("\t");

            // hash block size
            str.push_str(&patch.hash_block_size.to_string());
            str.push_str("\t");

            // hashes
            str.push_str(&patch.hashes[0]);
            for hash in &patch.hashes[1..] {
                str.push_str(",");
                str.push_str(&hash);
            }
            str.push_str("\t");

            // url
            str.push_str(&patch.url);
            str.push_str("\r\n");
        }

        str.push_str("--");
        str.push_str(&self.id);
        str.push_str("--\r\n");

        str
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_output() {
        let test_case = "--477D80B1_38BC_41d4_8B48_5273ADB89CAC\r\nContent-Type: application/octet-stream\r\nContent-Location: \
        ffxivpatch/4e9a232b/metainfo/2023.07.26.0000.0000.http\r\nX-Patch-Length: \
        1664916486\r\n\r\n1479062470\t44145529682\t71\t11\t2023.09.15.0000.0000\tsha1\t50000000\t1c66becde2a8cf26a99d0fc7c06f15f8bab2d87c,\
        950725418366c965d824228bf20f0496f81e0b9a,cabef48f7bf00fbf18b72843bdae2f61582ad264,53608de567b52f5fdb43fdb8b623156317e26704,\
        f0bc06cabf9ff6490f36114b25f62619d594dbe8,3c5e4b962cd8445bd9ee29011ecdb331d108abd8,88e1a2a322f09de3dc28173d4130a2829950d4e0,\
        1040667917dc99b9215dfccff0e458c2e8a724a8,149c7e20e9e3e376377a130e0526b35fd7f43df2,1bb4e33807355cdf46af93ce828b6e145a9a8795,\
        a79daff43db488f087da8e22bb4c21fd3a390f3c,6b04fadb656d467fb8318eba1c7f5ee8f030d967,a6641e1c894db961a49b70fda2b0d6d87be487a7,\
        edf419de49f42ef19bd6814f8184b35a25e9e977,c1525c4df6001b66b575e2891db0284dc3a16566,01b7628095b07fa3c9c1aed2d66d32d118020321,\
        991b137ea0ebb11bd668f82149bc2392a4cbcf52,ad3f74d4fca143a6cf507fc859544a4bcd501d85,936a0f1711e273519cae6b2da0d8b435fe6aa020,\
        023f19d8d8b3ecaaf865e3170e8243dd437a384c,2d9e934de152956961a849e81912ca8d848265ca,8e32f9aa76c95c60a9dbe0967aee5792b812d5ec,\
        dee052b9aa1cc8863efd61afc63ac3c2d56f9acc,fa81225aea53fa13a9bae1e8e02dea07de6d7052,59b24693b1b62ea1660bc6f96a61f7d41b3f7878,\
        349b691db1853f6c0120a8e66093c763ba6e3671,4561eb6f954d80cdb1ece3cc4d58cbd864bf2b50,de94175c4db39a11d5334aefc7a99434eea8e4f9,\
        55dd7215f24441d6e47d1f9b32cebdb041f2157f,2ca09db645cfeefa41a04251dfcb13587418347a\thttp://patch-dl.ffxiv.com/game/4e9a232b/\
        D2023.09.15.0000.0000.patch\r\n--477D80B1_38BC_41d4_8B48_5273ADB89CAC--\r\n";

        let patch_list = PatchList {
            id: "477D80B1_38BC_41d4_8B48_5273ADB89CAC".to_string(),
            patches: vec![
                PatchEntry {
                    url: "http://patch-dl.ffxiv.com/game/4e9a232b/D2023.09.15.0000.0000.patch".to_string(),
                    version: "2023.09.15.0000.0000".to_string(),
                    hash_block_size: 50000000,
                    length: 1479062470,
                    hashes: vec![
                        "1c66becde2a8cf26a99d0fc7c06f15f8bab2d87c".to_string(),
                        "950725418366c965d824228bf20f0496f81e0b9a".to_string(),
                        "cabef48f7bf00fbf18b72843bdae2f61582ad264".to_string(),
                        "53608de567b52f5fdb43fdb8b623156317e26704".to_string(),
                        "f0bc06cabf9ff6490f36114b25f62619d594dbe8".to_string(),
                        "3c5e4b962cd8445bd9ee29011ecdb331d108abd8".to_string(),
                        "88e1a2a322f09de3dc28173d4130a2829950d4e0".to_string(),
                        "1040667917dc99b9215dfccff0e458c2e8a724a8".to_string(),
                        "149c7e20e9e3e376377a130e0526b35fd7f43df2".to_string(),
                        "1bb4e33807355cdf46af93ce828b6e145a9a8795".to_string(),
                        "a79daff43db488f087da8e22bb4c21fd3a390f3c".to_string(),
                        "6b04fadb656d467fb8318eba1c7f5ee8f030d967".to_string(),
                        "a6641e1c894db961a49b70fda2b0d6d87be487a7".to_string(),
                        "edf419de49f42ef19bd6814f8184b35a25e9e977".to_string(),
                        "c1525c4df6001b66b575e2891db0284dc3a16566".to_string(),
                        "01b7628095b07fa3c9c1aed2d66d32d118020321".to_string(),
                        "991b137ea0ebb11bd668f82149bc2392a4cbcf52".to_string(),
                        "ad3f74d4fca143a6cf507fc859544a4bcd501d85".to_string(),
                        "936a0f1711e273519cae6b2da0d8b435fe6aa020".to_string(),
                        "023f19d8d8b3ecaaf865e3170e8243dd437a384c".to_string(),
                        "2d9e934de152956961a849e81912ca8d848265ca".to_string(),
                        "8e32f9aa76c95c60a9dbe0967aee5792b812d5ec".to_string(),
                        "dee052b9aa1cc8863efd61afc63ac3c2d56f9acc".to_string(),
                        "fa81225aea53fa13a9bae1e8e02dea07de6d7052".to_string(),
                        "59b24693b1b62ea1660bc6f96a61f7d41b3f7878".to_string(),
                        "349b691db1853f6c0120a8e66093c763ba6e3671".to_string(),
                        "4561eb6f954d80cdb1ece3cc4d58cbd864bf2b50".to_string(),
                        "de94175c4db39a11d5334aefc7a99434eea8e4f9".to_string(),
                        "55dd7215f24441d6e47d1f9b32cebdb041f2157f".to_string(),
                        "2ca09db645cfeefa41a04251dfcb13587418347a".to_string()
                    ],
                }
            ]
        };

        assert_eq!(patch_list.to_string(), test_case);
    }
}