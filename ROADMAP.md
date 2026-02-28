# AtChef ROADMAP

## Completed ✅

### Authentication & OAuth
- [x] OAuth login flow with Bluesky/ATProto
- [x] DPoP (Demonstrating Proof-of-Possession) implementation
- [x] Token refresh handling
- [x] **FIXED**: PDS URL discovery (was using auth server instead of actual PDS)
- [x] **FIXED**: DPoP nonce handling for XRPC requests (was failing with `use_dpop_nonce` error)

### Recipe Creation
- [x] Recipe creation form
- [x] Create recipe records on user's PDS
- [x] Recipe lexicon definition (`eu.atchef.recipe`)

## In Progress / Known Issues 🔧

### Recipe Viewing
- [ ] **NOT IMPLEMENTED**: Recipe viewing from PDS
  - Currently uses mock data (`get_mock_recipe_detail`)
  - Needs to fetch actual records from user's PDS using `agent.repo().getRecord()`
  - Parse and display the recipe data

### Recipe Listing
- [ ] **NOT IMPLEMENTED**: List recipes from PDS
  - Currently shows mock recipes
  - Should fetch from `com.atproto.repo.listRecords` with collection `eu.atchef.recipe`

## Technical Debt / Improvements 📝

### DPoP Implementation
- [ ] Store DPoP nonce across requests for performance (avoid retry on every request)
- [ ] Handle nonce expiration
- [ ] Add support for DPoP nonces to other HTTP methods (GET, PUT, DELETE)

### Error Handling
- [ ] Better error messages for users
- [ ] Handle network failures gracefully
- [ ] Session expiry handling with redirect to login

### Features to Add
- [ ] Recipe editing
- [ ] Recipe deletion
- [ ] Image upload for recipes
- [ ] Profile page showing user's recipes
- [ ] Recipe search/discovery
- [ ] Social features (likes, reposts)
- [ ] Recipe sharing via ATProtocol

## Recent Changes

### 2026-02-01
1. **Fixed InvalidToken error**: Changed `pds_url` to use actual PDS instead of authorization server
2. **Fixed use_dpop_nonce error**: Implemented DPoP nonce retry logic in XRPC client
3. **Added comprehensive logging**: Debug logging throughout auth flow for troubleshooting
4. **Recipe creation now works**: Successfully creates records on Bluesky PDS

## Architecture Notes

### DPoP Flow
1. Client generates DPoP keypair (P-256 EC)
2. On token exchange: Server may require nonce → client retries with nonce
3. On XRPC requests: Server may require nonce → client extracts from `DPoP-Nonce` header and retries
4. DPoP proof includes: `jti` (unique ID), `htm` (HTTP method), `htu` (URL), `iat` (timestamp), `ath` (access token hash), optional `nonce`

### Session Flow
1. `PendingAuth` stored during OAuth flow
2. On callback: Exchange code for tokens, fetch profile, create `AuthenticatedUser`
3. `AuthenticatedUser` stored in session with: DID, handle, tokens, DPoP keys, PDS URL
4. `DpopSession` created per-request to generate auth headers

## Debugging

To enable debug logging:
```bash
RUST_LOG=debug cargo run
```

Key log messages:
- `Generating auth headers:` - Shows PDS URL and token being used
- `DPoP proof claims:` - Shows htm, htu, iat, nonce, ath
- `XRPC POST request:` - Shows endpoint and full URL
- `XRPC POST response status:` - Shows HTTP status
- `Server requires DPoP nonce:` - Shows extracted nonce from 401 response
- `Retrying with DPoP nonce:` - Shows retry attempt
