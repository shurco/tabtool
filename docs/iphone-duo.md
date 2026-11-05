# iPhone Duo

TabTool targets the iOS 27.1+ SDK tier and supports both iPhone Duo displays.
The app is not a stretched phone layout: it reads the available size and lets
the system navigation containers place their bars.

## What the app does

- Uses `NavigationStack`, `List`, `ScrollView`, sheets, alerts, and menus, so
  the system can move the toolbar items to the vertical bar on the outer display
  and on the open inner display in landscape.
- Reads no interface orientation, device model, or main-screen size for layout.
  A fold, a rotation, iPhone Mirroring, or Split View only changes the size the
  screen is given.
- Sets every toolbar button with both a title and an image, so the title remains
  available in the system overflow menu.
- Keeps the grid and its pinned header and row-number cells inside the SwiftUI
  safe area. The collection view reloads its pinned chrome on every bounds
  change.
- Draws the cell hairline from the local `traitCollection.displayScale`, so the
  line stays one physical pixel on either display.
- Leaves a gap in the selection action bar when the inner display reports an
  active vertical division region (a partial fold), so no button sits in the
  curve. On flat and tabletop poses the bar spans the full width as usual.

## Pose checklist

Run the app in the iPhone Duo simulator in Device Hub. Open and close the device,
drag the hinge through the partial-fold range, rotate, and try Split View on
both sides of the display.

| Check | Closed | Book (partial fold) | Open flat | Tabletop | Split View |
| --- | --- | --- | --- | --- | --- |
| The app fills the available area with no letterboxing | yes | yes | yes | yes | yes |
| Toolbar items are reachable in the vertical bar | yes | yes | yes | depends on pose | depends on side |
| No button lands in the fold | n/a | yes | n/a | n/a | n/a |
| The open document and its selection survive the resize | yes | yes | yes | yes | yes |
| The grid header and row numbers stay pinned | yes | yes | yes | yes | yes |
| The selection action bar is inside the safe area | yes | yes | yes | yes | yes |

The UI smoke test covers creating a document and landing in the editor:

```sh
xcodebuild \
  -project app/apple/TabTool.xcodeproj \
  -scheme TabToolIOS \
  -destination 'platform=iOS Simulator,name=iPhone Duo' \
  -only-testing:TabToolIOSUITests \
  test
```

The test is functional. A scene screenshot does not include the hardware fold,
so verify the partial-fold layout in Device Hub before release.

## App Store

- Capture screenshots in the closed, open, and tabletop poses before publishing.
  Use the current App Store Connect specifications for the required sizes.
- Uploading iPhone Duo screenshots adds the optimized-for-iPhone-Duo badge to the
  product page.
- Use the App Store Connect preview to check how the assets render on the device.
- When submitting a featuring nomination, mention that the app is optimized for
  iPhone Duo and supports every device pose.
