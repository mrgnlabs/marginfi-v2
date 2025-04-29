# Rust Types for Marginfi-v2

Want to use the Mrgnlend types without importing the entire program? Look no further, this is just the types and a few helper functions, with the bare minimum dependencies beyond that.

Notes: 
* Pubkey is a stub (to avoid any crypto dependencies), remember to cast to the real Pubkey if you need anything beyond eq.
* Discriminators are available as a const for each on-chain struct.