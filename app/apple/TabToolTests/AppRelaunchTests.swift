import Testing
@testable import TabTool

@Suite @MainActor struct AppRelaunchTests {
    @Test func aQuitAttemptAfterARequestIsARelaunch() {
        AppRelaunch.request(quit: {})
        AppRelaunch.quitAttemptStarted()
        #expect(AppRelaunch.isPending)
    }

    @Test func aLaterQuitWithoutARequestDoesNotRelaunch() {
        AppRelaunch.request(quit: {})
        AppRelaunch.quitAttemptStarted()
        AppRelaunch.quitAttemptStarted()
        #expect(!AppRelaunch.isPending)
    }
}
