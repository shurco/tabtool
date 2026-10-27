import AppKit
import XCTest

final class TabToolUITests: XCTestCase {
    /// Launches a clean app instance.
    ///
    /// The document app restores windows on relaunch. Without ignoring the
    /// saved state, windows left by earlier tests make identifiers ambiguous.
    @MainActor
    private func launchApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments += [
            "-ApplePersistenceIgnoreState", "YES",
            "-NSQuitAlwaysKeepsWindows", "NO"
        ]
        app.launch()
        return app
    }

    @MainActor
    func testLaunchStartsTheApp() {
        let app = launchApp()
        XCTAssertEqual(app.state, .runningForeground)
    }

    @MainActor
    func testNewDocumentShowsGrid() {
        let app = launchApp()
        // A windowless document app may not receive Cmd-N, so use the menu
        // command to create the untitled document.
        app.menuBars.menuItems["New"].click()
        XCTAssertTrue(app.tables["grid"].waitForExistence(timeout: 5))
    }

    /// Opens a document through Launch Services, exercising the real file-open
    /// path without automating the system open panel.
    @MainActor
    private func openDocument(at url: URL) throws {
        guard let appURL = NSWorkspace.shared.urlForApplication(
            withBundleIdentifier: "app.shurco.tabtool"
        ) else {
            throw XCTSkip("TabTool.app is not registered with Launch Services")
        }
        let opened = expectation(description: "Open \(url.lastPathComponent)")
        NSWorkspace.shared.open(
            [url],
            withApplicationAt: appURL,
            configuration: NSWorkspace.OpenConfiguration()
        ) { _, error in
            XCTAssertNil(error)
            opened.fulfill()
        }
        wait(for: [opened], timeout: 10)
    }

    private func makeFixture(_ text: String, name: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("TabToolUITests", isDirectory: true)
        try FileManager.default.createDirectory(at: directory,
                                                withIntermediateDirectories: true)
        let url = directory.appendingPathComponent(name)
        try text.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// The format-bar visibility is a shared user default. A previous run or
    /// a failed test can leave it hidden, so reveal it before querying controls.
    @MainActor
    private func showFormatBar(in app: XCUIApplication) {
        if app.popUpButtons["format-encoding"].waitForExistence(timeout: 2) {
            return
        }
        app.menuBars.menuBarItems["View"].click()
        app.menuItems["Show Format Bar"].click()
        XCTAssertTrue(app.popUpButtons["format-encoding"].waitForExistence(timeout: 5))
    }

    @MainActor
    func testOpenFixtureShowsData() throws {
        let fixture = try makeFixture("name,age\nRex,5\nCaesar,3\n",
                                      name: "open-fixture.csv")
        let app = launchApp()
        try openDocument(at: fixture)

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 10))
        let status = app.staticTexts["status-text"]
        XCTAssertTrue(status.waitForExistence(timeout: 10))
        XCTAssertTrue((status.value as? String ?? "").contains("2 rows"))
        XCTAssertGreaterThan(grid.descendants(matching: .tableRow).count, 0)
    }

    @MainActor
    func testAddRowAndColumnButtonsChangeGrid() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let addRow = app.buttons["add-row"]
        let addColumn = app.buttons["add-column"]
        let delete = app.buttons["delete-selection"]
        XCTAssertTrue(addRow.waitForExistence(timeout: 5))
        XCTAssertTrue(addColumn.waitForExistence(timeout: 5))
        XCTAssertTrue(delete.exists)
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 1)
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 3)
        XCTAssertFalse(delete.isEnabled)

        addRow.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)
        XCTAssertTrue(delete.isEnabled)

        addColumn.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 4)
        XCTAssertTrue(delete.isEnabled)
        XCTAssertTrue(grid.exists)
    }

    @MainActor
    func testToolbarContextMenusAddAboveAndLeft() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let addRow = app.buttons["add-row"]
        XCTAssertTrue(addRow.waitForExistence(timeout: 5))
        addRow.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).rightClick()

        // The Table menu now exposes commands with the same titles, so scope
        // the query to the context menu attached to the document window.
        let contextMenu = app.windows.firstMatch.menus.firstMatch
        let addAbove = contextMenu.menuItems["Add Row Above"]
        XCTAssertTrue(addAbove.waitForExistence(timeout: 5))
        addAbove.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)

        let addColumn = app.buttons["add-column"]
        addColumn.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).rightClick()
        let addLeft = app.windows.firstMatch.menus.firstMatch.menuItems["Add Column Left"]
        XCTAssertTrue(addLeft.waitForExistence(timeout: 5))
        addLeft.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 4)
    }

    @MainActor
    func testCopyPasteThroughEditMenu() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let cell = grid.descendants(matching: .cell).element(boundBy: 0)
        XCTAssertTrue(cell.waitForExistence(timeout: 5))
        cell.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()

        app.menuBars.menuItems["Copy"].click()
        app.menuBars.menuItems["Paste"].click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)
    }

    @MainActor
    func testDeleteIsDisabledForTheOnlyColumn() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))
        let delete = app.buttons["delete-selection"]

        // Remove two of the three new-document columns.
        for header in ["C", "B"] {
            let button = grid.buttons[header]
            XCTAssertTrue(button.waitForExistence(timeout: 5))
            button.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
            XCTAssertTrue(delete.isEnabled)
            delete.click()
        }
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 1)

        // The only column stays selected but cannot be deleted.
        let onlyHeader = grid.buttons["A"]
        XCTAssertTrue(onlyHeader.waitForExistence(timeout: 5))
        onlyHeader.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        XCTAssertFalse(delete.isEnabled)
    }

    @MainActor
    func testFormatBarShowsControlsAndStatus() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))
        showFormatBar(in: app)

        for identifier in ["format-encoding", "format-separator", "format-quote",
                           "format-decimal", "format-header"] {
            let control = app.descendants(matching: .any)[identifier].firstMatch
            XCTAssertTrue(control.waitForExistence(timeout: 5),
                          "Missing format control \(identifier)")
        }

        XCTAssertEqual(app.popUpButtons["format-encoding"].value as? String,
                       "Unicode (UTF-8)")
        XCTAssertEqual(app.popUpButtons["format-separator"].value as? String,
                       "Comma (,)")
        XCTAssertEqual(app.popUpButtons["format-quote"].value as? String,
                       "Doubled")
        XCTAssertEqual(app.popUpButtons["format-decimal"].value as? String,
                       "Dot (.)")
        XCTAssertTrue(app.checkBoxes["format-header"].isEnabled)

        let status = app.staticTexts["status-text"]
        XCTAssertTrue(status.waitForExistence(timeout: 5))
        let summary = status.value as? String ?? ""
        XCTAssertTrue(summary.contains("3 columns"))
        XCTAssertTrue(summary.contains("Unicode (UTF-8)"))
        XCTAssertTrue(summary.contains("Comma"))

        // Separator changes on an untitled document update the configuration
        // in place and are reflected by the status line.
        app.popUpButtons["format-separator"].click()
        let semicolon = app.menuItems["Semicolon (;)"]
        XCTAssertTrue(semicolon.waitForExistence(timeout: 5))
        semicolon.click()
        XCTAssertEqual(app.popUpButtons["format-separator"].value as? String,
                       "Semicolon (;)")
        XCTAssertTrue((status.value as? String ?? "").contains("Semicolon"))

        // Toggling Header moves the only row across the header boundary.
        let header = app.checkBoxes["format-header"]
        header.click()
        XCTAssertTrue((status.value as? String ?? "").contains("0 rows"))
        header.click()
        XCTAssertTrue((status.value as? String ?? "").contains("1 row"))
    }

    @MainActor
    func testTableMenuAddsAndDeletesRows() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        app.menuBars.menuBarItems["Table"].click()
        let addRow = app.menuItems["Add Row Below"]
        XCTAssertTrue(addRow.waitForExistence(timeout: 5))
        XCTAssertTrue(addRow.isEnabled)
        addRow.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)

        app.menuBars.menuBarItems["Table"].click()
        let deleteRows = app.menuItems["Delete Row(s)"]
        XCTAssertTrue(deleteRows.waitForExistence(timeout: 5))
        XCTAssertTrue(deleteRows.isEnabled)
        deleteRows.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 1)

        let header = grid.buttons["B"]
        XCTAssertTrue(header.waitForExistence(timeout: 5))
        header.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()

        app.menuBars.menuBarItems["Table"].click()
        let deleteColumns = app.menuItems["Delete Column(s)"]
        XCTAssertTrue(deleteColumns.waitForExistence(timeout: 5))
        XCTAssertTrue(deleteColumns.isEnabled)
        deleteColumns.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 2)

        app.menuBars.menuBarItems["View"].click()
        XCTAssertTrue(app.menuItems["Show Format Bar"].waitForExistence(timeout: 5))

        app.menuBars.menuBarItems["File"].click()
        let revert = app.menuItems["Revert to Saved"]
        XCTAssertTrue(revert.waitForExistence(timeout: 5))
        XCTAssertFalse(revert.isEnabled)
        let export = app.menuItems["Export…"]
        XCTAssertTrue(export.waitForExistence(timeout: 5))
        XCTAssertTrue(export.isEnabled)
    }

    @MainActor
    func testNeedsAttentionBannerOffersUndo() {
        let app = launchApp()
        defer {
            NSPasteboard.general.clearContents()
            // Leave the shared format-bar preference visible for later tests.
            if !app.popUpButtons["format-encoding"].exists {
                app.menuBars.menuBarItems["View"].click()
                if app.menuItems["Show Format Bar"].waitForExistence(timeout: 2) {
                    app.menuItems["Show Format Bar"].click()
                }
            }
        }
        app.menuBars.menuItems["New"].click()

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))
        showFormatBar(in: app)

        app.popUpButtons["format-encoding"].click()
        let latin1 = app.menuItems["Western (Windows Latin 1)"]
        if !latin1.waitForExistence(timeout: 2) {
            app.popUpButtons["format-encoding"].click()
        }
        XCTAssertTrue(latin1.waitForExistence(timeout: 5))
        latin1.click()

        // Hide the bar so "Show Format Controls" has something to reveal.
        app.menuBars.menuBarItems["View"].click()
        app.menuItems["Show Format Bar"].click()
        XCTAssertFalse(app.popUpButtons["format-encoding"].waitForExistence(timeout: 1))

        NSPasteboard.general.clearContents()
        XCTAssertTrue(NSPasteboard.general.setString("→", forType: .string))

        let cell = grid.descendants(matching: .cell).element(boundBy: 0)
        XCTAssertTrue(cell.waitForExistence(timeout: 5))
        cell.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        app.menuBars.menuItems["Paste"].click()

        let banner = app.descendants(matching: .any)["needs-attention-banner"].firstMatch
        XCTAssertTrue(banner.waitForExistence(timeout: 5))

        // Autosave is suspended by the typed snapshot error. Give it a moment
        // to prove it does not add a modal error on every retry.
        Thread.sleep(forTimeInterval: 2)
        XCTAssertEqual(app.alerts.count, 0)
        XCTAssertEqual(app.sheets.count, 0)

        let showControls = app.buttons["show-format-controls"]
        XCTAssertTrue(showControls.waitForExistence(timeout: 5))
        showControls.click()
        XCTAssertTrue(app.popUpButtons["format-encoding"].waitForExistence(timeout: 5))

        let undo = app.buttons["undo-last-edit"]
        XCTAssertTrue(undo.waitForExistence(timeout: 5))
        XCTAssertTrue(undo.isEnabled)
        undo.click()
        XCTAssertFalse(banner.waitForExistence(timeout: 1))
    }

    /// Types through the real field editor, then uses Add Row and Save while
    /// an edit is still in progress. The text must be committed against the
    /// original row and column, survive the save, and be present after reopen.
    @MainActor
    func testTypedCellSurvivesAddRowSaveAndReopen() throws {
        NSPasteboard.general.clearContents()
        let fixture = try makeFixture("a,b\nc,d\n",
                                      name: "edit-save-\(UUID().uuidString).csv")
        defer { try? FileManager.default.removeItem(at: fixture) }
        let app = launchApp()
        try openDocument(at: fixture)

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 10))
        let rows = grid.descendants(matching: .tableRow)
        let firstRow = rows.element(boundBy: 0)
        XCTAssertTrue(firstRow.waitForExistence(timeout: 5))
        NSPasteboard.general.clearContents()
        XCTAssertTrue(NSPasteboard.general.setString("typed", forType: .string))
        firstRow.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
        app.typeKey(.return, modifierFlags: [])
        Thread.sleep(forTimeInterval: 0.5)
        app.menuBars.menuItems["Paste"].click()
        Thread.sleep(forTimeInterval: 0.5)

        // Add Row is a structural mutation: it must commit the open editor
        // first, then keep the text on the original row.
        app.buttons["add-row"].click()
        let addedRow = rows.element(boundBy: 1)
        XCTAssertTrue(addedRow.waitForExistence(timeout: 5),
                      "Add Row did not run while the field editor was open")
        NSPasteboard.general.clearContents()
        XCTAssertTrue(NSPasteboard.general.setString("saved", forType: .string))
        addedRow.coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
        app.typeKey(.return, modifierFlags: [])
        Thread.sleep(forTimeInterval: 0.5)
        app.menuBars.menuItems["Paste"].click()
        Thread.sleep(forTimeInterval: 0.5)
        app.activate()
        app.typeKey("s", modifierFlags: .command)
        XCTAssertTrue(waitForFile(fixture, containing: "saved", timeout: 10),
                      "Save did not commit the in-progress field editor")

        app.typeKey("w", modifierFlags: .command)
        XCTAssertTrue(grid.waitForNonExistence(timeout: 5))

        try openDocument(at: fixture)
        let reopened = app.tables["grid"]
        XCTAssertTrue(reopened.waitForExistence(timeout: 10))
        let status = app.staticTexts["status-text"]
        XCTAssertTrue(status.waitForExistence(timeout: 10))
        XCTAssertTrue((status.value as? String ?? "").contains("2 rows"))

        // The file and the reopened table agree on the intended cells:
        // header a,b; the typed cell in row 0; the saved cell in row 1.
        let saved = try String(contentsOf: fixture, encoding: .utf8)
        XCTAssertEqual(saved, "\"a\",\"b\"\n\"typed\",\"d\"\n\"saved\",\"\"")

        // The focused file-backed document enables Revert to Saved.
        app.menuBars.menuBarItems["File"].click()
        let revert = app.menuItems["Revert to Saved"]
        XCTAssertTrue(revert.waitForExistence(timeout: 5))
        XCTAssertTrue(revert.isEnabled)
        app.typeKey(.escape, modifierFlags: [])
    }

    /// A file-backed document in needsAttention must suspend autosave without
    /// spamming modal alerts; the persistent banner is the only error UI.
    @MainActor
    func testFileBackedNeedsAttentionSuspendsAutosaveQuietly() throws {
        let fixture = try makeFixture("a,b\nc,d\n",
                                      name: "needs-attention-\(UUID().uuidString).csv")
        defer { try? FileManager.default.removeItem(at: fixture) }
        let app = launchApp()
        try openDocument(at: fixture)
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 10))
        showFormatBar(in: app)

        app.popUpButtons["format-encoding"].click()
        let latin1 = app.menuItems["Western (Windows Latin 1)"]
        XCTAssertTrue(latin1.waitForExistence(timeout: 5))
        latin1.click()
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        NSPasteboard.general.clearContents()
        XCTAssertTrue(NSPasteboard.general.setString("→", forType: .string))
        let cell = grid.descendants(matching: .cell).element(boundBy: 0)
        XCTAssertTrue(cell.waitForExistence(timeout: 5))
        cell.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        app.menuBars.menuItems["Paste"].click()

        let banner = app.descendants(matching: .any)["needs-attention-banner"].firstMatch
        XCTAssertTrue(banner.waitForExistence(timeout: 5))

        // Autosave has a file URL but cannot serialize this table. It must stay
        // suspended instead of presenting retry alerts or writing a partial file.
        Thread.sleep(forTimeInterval: 3)
        XCTAssertEqual(app.alerts.count, 0)
        XCTAssertEqual(app.sheets.count, 0)

        // Explicit Save is blocked by the same typed snapshot error: no
        // partial file is written and the banner remains the error UI.
        app.activate()
        Thread.sleep(forTimeInterval: 0.5)
        app.typeKey("s", modifierFlags: .command)
        let sheet = app.sheets.firstMatch
        if !sheet.waitForExistence(timeout: 2) {
            // A key event can be lost when the runner briefly takes focus.
            app.typeKey("s", modifierFlags: .command)
        }
        XCTAssertTrue(sheet.waitForExistence(timeout: 5),
                      "Explicit Save must report the blocked write")
        if sheet.buttons.firstMatch.exists {
            sheet.buttons.firstMatch.click()
        }
        XCTAssertTrue(sheet.waitForNonExistence(timeout: 5))

        let contents = try String(contentsOf: fixture, encoding: .utf8)
        XCTAssertFalse(contents.contains("→"))

        // The encoding change is a real document mutation and may autosave
        // the still-valid table before the paste. Compare logical cells, not
        // the writer's quoting or a trailing newline: a quoted two-row table
        // is as correct as the original unquoted file.
        let normalized = contents
            .replacingOccurrences(of: "\"", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let lines = normalized.split(separator: "\n", omittingEmptySubsequences: false)
        let cells = lines.map {
            $0.trimmingCharacters(in: .whitespaces)
                .split(separator: ",", omittingEmptySubsequences: false)
                .map(String.init)
        }
        XCTAssertEqual(cells, [["a", "b"], ["c", "d"]],
                       "Autosave must keep the complete logical table")
    }

    /// Polls a file until it contains `text`; saves are asynchronous across
    /// the test-runner and app processes.
    private func waitForFile(_ url: URL, containing text: String,
                             timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let contents = try? String(contentsOf: url, encoding: .utf8),
               contents.contains(text) {
                return true
            }
            Thread.sleep(forTimeInterval: 0.1)
        }
        return false
    }

    @MainActor
    func testAboutShowsVersionAndDescription() {
        let app = launchApp()

        app.menuBars.menuBarItems["TabTool"].click()
        let about = app.menuBars.menuItems["About TabTool"].firstMatch
        XCTAssertTrue(about.waitForExistence(timeout: 5))
        about.click()

        XCTAssertTrue(app.staticTexts["TabTool"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["A simple CSV editor for macOS."].waitForExistence(timeout: 5))
    }

    @MainActor
    func testHeaderMenuSortsRows() throws {
        let fixture = try makeFixture("pear,1\napple,2\nzebra,3\n",
                                      name: "sort-fixture-\(UUID().uuidString).csv")
        defer { try? FileManager.default.removeItem(at: fixture) }
        let app = launchApp()
        try openDocument(at: fixture)

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 10))
        let header = grid.buttons["A"]
        XCTAssertTrue(header.waitForExistence(timeout: 5))
        header.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).rightClick()
        let sortDescending = app.menuItems["sortDescending:"]
        XCTAssertTrue(sortDescending.waitForExistence(timeout: 5))
        sortDescending.click()

        let firstCell = grid.descendants(matching: .tableRow).element(boundBy: 0)
            .textFields.element(boundBy: 0)
        XCTAssertTrue(firstCell.waitForExistence(timeout: 5))
        let showsZebra = NSPredicate(format: "value == %@", "zebra")
        let wait = XCTWaiter().wait(
            for: [XCTNSPredicateExpectation(predicate: showsZebra, object: firstCell)],
            timeout: 5)
        XCTAssertEqual(wait, .completed, "first cell shows \(firstCell.value as? String ?? "nil")")
    }

    @MainActor
    func testTableMenuMovesRowDown() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        app.menuBars.menuBarItems["Table"].click()
        app.menuItems["Add Row Below"].click()
        let rows = grid.descendants(matching: .tableRow)
        XCTAssertEqual(rows.count, 2)

        NSPasteboard.general.clearContents()
        XCTAssertTrue(NSPasteboard.general.setString("x", forType: .string))
        rows.element(boundBy: 0).coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
        app.typeKey(.return, modifierFlags: [])
        Thread.sleep(forTimeInterval: 0.5)
        app.menuBars.menuItems["Paste"].click()
        Thread.sleep(forTimeInterval: 0.5)
        app.typeKey(.return, modifierFlags: [])

        rows.element(boundBy: 0).coordinate(withNormalizedOffset: CGVector(dx: 0.1, dy: 0.5)).click()
        app.menuBars.menuBarItems["Table"].click()
        let moveDown = app.menuItems["Move Row Down"]
        XCTAssertTrue(moveDown.waitForExistence(timeout: 5))
        XCTAssertTrue(moveDown.isEnabled)
        moveDown.click()

        let moved = rows.element(boundBy: 1).textFields.element(boundBy: 0)
        XCTAssertTrue(moved.waitForExistence(timeout: 5))
        let showsX = NSPredicate(format: "value == %@", "x")
        let wait = XCTWaiter().wait(
            for: [XCTNSPredicateExpectation(predicate: showsX, object: moved)], timeout: 5)
        XCTAssertEqual(wait, .completed, "row 1 shows \(moved.value as? String ?? "nil")")
    }

    @MainActor
    func testHeaderClickSelectsAColumn() {
        let app = launchApp()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let delete = app.buttons["delete-selection"]
        XCTAssertFalse(delete.isEnabled)

        let header = grid.buttons["B"]
        XCTAssertTrue(header.waitForExistence(timeout: 5))
        header.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        XCTAssertTrue(delete.isEnabled)

        // A column selection deletes a column; a row selection would remove
        // the only data row instead.
        delete.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 2)
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 1)
    }
}
