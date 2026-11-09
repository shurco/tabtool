import Foundation
import Testing
@testable import TabTool

@Suite struct ReviewPromptTests {
    private let day: TimeInterval = 86_400

    private func date(_ days: Int) -> Date {
        Date(timeIntervalSince1970: 1_800_000_000 + Double(days) * day)
    }

    private var calendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 0)!
        return calendar
    }

    private func policy(_ store: TestDefaults, day: Int, version: String = "1.0.1") -> ReviewPromptPolicy {
        ReviewPromptPolicy(defaults: store.defaults, calendar: calendar,
                           now: date(day), version: version)
    }

    @Test func firstLaunchIsRecordedOnce() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 0).recordLaunch()

        #expect(store.defaults.integer(forKey: Preferences.Key.reviewActiveDays) == 1)
        #expect(store.defaults.object(forKey: Preferences.Key.reviewFirstLaunch) as? Date == date(0))
    }

    @Test func distinctDaysCount() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 1).recordLaunch()
        policy(store, day: 2).recordLaunch()
        policy(store, day: 2).recordLaunch()

        #expect(store.defaults.integer(forKey: Preferences.Key.reviewActiveDays) == 3)
    }

    @Test func sevenDaysAndThreeDaysAndFiveEngagementsAreRequired() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 1).recordLaunch()
        policy(store, day: 2).recordLaunch()
        for _ in 0..<5 { policy(store, day: 2).recordEngagement() }

        #expect(!policy(store, day: 6).canPrompt)
        #expect(policy(store, day: 7).canPrompt)

        let twoDays = TestDefaults()
        policy(twoDays, day: 0).recordLaunch()
        policy(twoDays, day: 1).recordLaunch()
        for _ in 0..<5 { policy(twoDays, day: 1).recordEngagement() }
        #expect(!policy(twoDays, day: 7).canPrompt)

        let fourEngagements = TestDefaults()
        policy(fourEngagements, day: 0).recordLaunch()
        policy(fourEngagements, day: 1).recordLaunch()
        policy(fourEngagements, day: 2).recordLaunch()
        for _ in 0..<4 { policy(fourEngagements, day: 2).recordEngagement() }
        #expect(!policy(fourEngagements, day: 7).canPrompt)
    }

    @Test func promptIsAllowedWhenEveryConditionHolds() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 1).recordLaunch()
        policy(store, day: 2).recordLaunch()
        for _ in 0..<5 { policy(store, day: 2).recordEngagement() }

        let ready = policy(store, day: 7)
        #expect(ready.canPrompt)
        #expect(ready.claimPrompt())
        #expect(!policy(store, day: 7).canPrompt)
    }

    @Test func promptIsClaimedOncePerVersion() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 1).recordLaunch()
        policy(store, day: 2).recordLaunch()
        for _ in 0..<5 { policy(store, day: 2).recordEngagement() }

        #expect(policy(store, day: 7).claimPrompt())
        #expect(!policy(store, day: 7).claimPrompt())
    }

    @Test func newVersionAllowsANewPromptAfterTheCooldown() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 1).recordLaunch()
        policy(store, day: 2).recordLaunch()
        for _ in 0..<5 { policy(store, day: 2).recordEngagement() }
        #expect(policy(store, day: 7, version: "1.0.1").claimPrompt())

        let nextVersion = policy(store, day: 8, version: "1.0.2")
        #expect(!nextVersion.canPrompt)
        #expect(policy(store, day: 97, version: "1.0.2").canPrompt)
    }

    @Test func explicitRateSuppressesThePrompt() {
        let store = TestDefaults()
        policy(store, day: 0).recordLaunch()
        policy(store, day: 1).recordLaunch()
        policy(store, day: 2).recordLaunch()
        for _ in 0..<5 { policy(store, day: 2).recordEngagement() }

        let ready = policy(store, day: 7)
        #expect(ready.canPrompt)
        ready.markPrompted()
        #expect(!policy(store, day: 7).canPrompt)
    }

    @Test func engagementIsCapped() {
        let store = TestDefaults()
        for _ in 0..<200 {
            policy(store, day: 2).recordEngagement()
        }
        #expect(store.defaults.integer(forKey: Preferences.Key.reviewEngagements) == 100)
    }
}
