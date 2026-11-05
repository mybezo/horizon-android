# Zalith controller port on the original Android launcher

Base: Vasysik/horizon-android `android`, commit `150a414b09a18c900489d9462a7fc035dae744b7`.
Controller source: MocLG/horizon-android `a69321a00de03daa5ab3a42397ceea83895d6733`.

This copies the controller/layout modules rather than updating MocLG's engine.
The original Vasysik launcher, native Activity, account/session implementation,
Android Keystore adapter, game content and rendering engine remain the base.
The launcher UI is unchanged. Shared UI code receives only the pixel drawing
primitives required by the controller shapes; existing UI calls are unchanged.

## Use

- Play using the original launcher and choose the Touch input mode.
- The floating H menu opens the controller editor and **Import Zalith JSON**.
- The original **Touch Controls** settings entry opens the new layout editor,
  including from the loading/title screen's settings.
- Import a controller JSON with Android's file picker. The original v12 JSON is
  validated and stored without conversion, at `controls/zalith.json` in the
  selected profile's game folder. Invalid/oversized imports keep the old layout.
- Edit buttons, joysticks, layers and styles, then Save. Back/Exit without Save
  discards the edit. Save errors leave the editor open.
- Opacity, visibility, size and floating menu position are saved separately in
  `controls/display.json`; they do not rewrite the imported layout.
- Android Files → Horizon → **Zalith Controls** exposes controller files for
  copying/exporting. Original profile boundaries and symlink checks still apply.

Button geometry and styles come from the Zalith layout. Original native touch
settings still control world gestures, camera sensitivity and gyroscope;
native button geometry rows do not rewrite the Zalith layout. This preserves
existing launcher UI pending a later UI redesign.

## Integration

Claimed controller fingers cannot hit the original hidden touch buttons.
Penetrable controls share camera movement with the original gesture path.
Unclaimed touches keep the original world and inventory handling. Hardware
key ownership, game rebindings, native mod key callbacks, mouse buttons and
held scroll events use the original engine's input paths. Render scale is
included in the controller density so hit tests and drawing share coordinates.
The original native auxiliary overlay is suppressed while the new layout owns
buttons, avoiding duplicate controls. Loss of focus or a switch away from Touch
releases held controller inputs.

The Android dev workflow now builds on every branch push. It retries SDK/NDK
installation and uses Gradle 8.14.5; Java 25 still prepares game resources and
Java 17 packages Android. Gradle cleanup switches back to Java 17. Preparation
uses the immutable engine-pinned Mojang version metadata, keeping its checksum
validation instead of following a republished live-catalog entry.

## Validation

- Android ARM64 compiler check passed: `cargo +1.95.0 check -p horizon-android --target aarch64-linux-android`.
- Seven executable Rust checks passed using the real parser/storage/document
  modules: original JSON byte preservation, edit-save roundtrip, rejection of
  invalid/oversized/non-UTF8 imports without replacing a layout, document/profile
  boundaries, and the supplied v12 layout (38 buttons, two layers).
- `python3 tools/test_zalith_controls.py` runs the reusable storage/document checks; `--layout PATH` additionally checks a supplied layout.
- Sixteen Android Python checks passed, including the pinned manifest regression.
- actionlint 1.7.7 validated both workflows; workflow/template pairs match.
- Public source boundary checks and archive integrity are checked before export.

No APK assembly, Microsoft sign-in, real-phone touch interaction or performance
measurement was run for this port. The compiler/storage checks do not establish
those runtime results. Test the newly built APK on a phone.
