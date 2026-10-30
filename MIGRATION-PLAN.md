# Nerevar-rs Migration Plan for TES3MP-NG

## Overview

This plan outlines the changes needed to migrate nerevar-rs from supporting the legacy TES3MP architecture (based on OpenMW 0.47) to the new tes3mp-ng architecture (rebased on OpenMW 0.51 with Lua scripting API).

## Key Architectural Changes in TES3MP-NG

| Aspect | Legacy TES3MP | TES3MP-NG |
|--------|---------------|-----------|
| Base version | OpenMW 0.47 | OpenMW 0.51 |
| Scripting | Custom C++ scripting | Lua via CoreScripts-NG |
| Config location | `Documents/My Games/OpenMW` | Portable: next to executables (`config/`) |
| User data location | `Documents/My Games/OpenMW` | Portable: next to executables (`userdata/`) |
| Client executable | `tes3mp_client.exe` | Likely `openmw-client.exe` or similar |
| Server executable | `tes3mp_server.exe` | Likely `openmw-server.exe` or similar |
| Chat key | F2 | F8 |
| Versioning | TES3MP scheme (0.8.x) | OpenMW scheme (0.51.x) |

## Migration Tasks

### Phase 1: Instance Data Layout Redesign

**Current state:** nerevar-rs assumes the legacy directory structure where config and userdata are stored in `Documents/My Games/OpenMW`.

**Required changes:**
- Update `instance_data` module to use portable directories next to executables
- Modify `create_instance_data_dir()` to create `config/` and `userdata/` subdirectories within the instance root
- Update all path resolution functions (`resolve_package_data_dir`, `instance_tes3mp_dir`) to reflect new layout

**Files affected:**
- `src-tauri/src/instance_data/mod.rs`
- `src-tauri/src/instance_data/commands.rs`
- `src-tauri/src/instance_setup.rs`

### Phase 2: Process Management Updates

**Current state:** `process_manager` launches `tes3mp_client.exe` and `tes3mp_server.exe` with legacy arguments.

**Required changes:**
- Update executable names to match tes3mp-ng (verify actual names from release)
- Modify launch arguments for OpenMW 0.51 compatibility
- Add support for Lua scripting API configuration if needed
- Update process detection and termination logic if executable names change

**Files affected:**
- `src-tauri/src/process_manager.rs`

### Phase 3: GitHub Release Download Updates

**Current state:** Downloads TES3MP releases from the legacy repository.

**Required changes:**
- Update repository URL to tes3mp-ng repo
- Modify asset naming patterns for release downloads
- Handle OpenMW-style versioning (0.51.x) in version comparison logic
- Update `github_getters` module accordingly

**Files affected:**
- `src-tauri/src/github_getters.rs`
- Any version parsing/comparison utilities

### Phase 4: Configuration File Generation

**Current state:** Generates `nerevar-launch.cfg` for legacy TES3MP.

**Required changes:**
- Update config format for OpenMW 0.51
- Add Lua scripting API configuration options if applicable
- Ensure compatibility with CoreScripts-NG server-side scripting
- Test chat key binding (F8 instead of F2) in generated configs

**Files affected:**
- `src-tauri/src/instance_data/mod.rs` (write_ephemeral_openmw_cfg function)
- Config template files if any exist

### Phase 5: Sync Architecture Updates

**Current state:** Manifest-based sync downloads mods and validates file integrity.

**Required changes:**
- Verify manifest format compatibility with tes3mp-ng
- Update download paths to use portable directories
- Ensure mod sync works with OpenMW 0.51's new mod handling
- Test CoreScripts-NG integration if server-side scripts are synced

**Files affected:**
- `src-tauri/src/sync_client/` (all files)
- Manifest validation logic

### Phase 6: Connection Management Updates

**Current state:** Handles adding, syncing, and launching instances with legacy architecture.

**Required changes:**
- Update connection commands to use new instance layout
- Modify sync triggers for portable directory structure
- Ensure launch flow works with updated process management
- Test end-to-end connection scenarios

**Files affected:**
- `src-tauri/src/connection/commands.rs`
- Related connection modules

### Phase 7: Frontend Updates (if needed)

**Current state:** TypeScript frontend communicates with Rust backend via Tauri commands.

**Required changes:**
- Update any hardcoded paths or assumptions about directory structure
- Modify UI if new configuration options are exposed (Lua scripting, etc.)
- Test all user flows with updated backend

**Files affected:**
- `src/` (TypeScript frontend)
- Any shared types between frontend and backend

## Testing Strategy

1. **Unit tests:** Update existing tests for new directory structure and executable names
2. **Integration tests:** Test full instance lifecycle (create → sync → launch → stop)
3. **Manual testing:** Verify chat key binding, Lua scripting integration, mod loading
4. **Regression testing:** Ensure backward compatibility or document breaking changes

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Executable names differ from expectations | High | Verify actual names from tes3mp-ng release before implementation |
| Config format incompatibility | Medium | Test thoroughly with OpenMW 0.51 documentation |
| Lua scripting API changes | Low-Medium | Monitor CoreScripts-NG development and update accordingly |

## Success Criteria

- [ ] nerevar-rs can create new tes3mp-ng instances with correct directory layout
- [ ] Instances can sync mods from remote servers successfully
- [ ] Client and server processes launch correctly with proper arguments
- [ ] Chat functionality works (F8 key binding)
- [ ] Lua scripting integration functions as expected
- [ ] All existing user flows continue to work or are documented as changed
- [ ] Synced instances should be entirely inherited. i.e. no configuration from end user accept the nerevar sync host and port. Instance name and description are inherited and the game address and port are passed on from the sync host
- [ ] Config shadowing can largely be done away with as we don't have to worry about the global cfg anymore. Portable config and user data allows us to setup a synced instance with the initial cfg overrides, and if the sync player then needs to change them for performance etc, they just can. We don't need to enforce it constantly and can just set the defaults from the sync host.
- [ ] Handle shader.yaml and other extra data sync
- [ ] Let nerevar handle generating the default cfg from the selected Morrowind install with adequate data= and content= entries for the base game files rather than the user having to run the openmw wizard.
- [ ] Backwards compatibility is not important




## Next Steps

1. Verify actual executable names and argument format from tes3mp-ng release
2. Review OpenMW 0.51 documentation for config format changes
3. Begin Phase 1 implementation (instance data layout)
4. Iterate through phases with testing at each stage