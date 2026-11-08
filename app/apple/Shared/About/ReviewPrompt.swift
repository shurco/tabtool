import Foundation

/// Decides when the app may ask for a rating.
///
/// The policy is deliberately conservative: a person must have used the app on
/// three distinct days over at least a week, and made five edits, saves, or
/// exports. The system still decides whether its sheet appears, so the app
/// records the version before asking and never asks twice for the same version.
///
/// The type is a value so tests can drive the clock and a private defaults
/// suite without touching `UserDefaults.standard`.
struct ReviewPromptPolicy {
    private let defaults: UserDefaults
    private let calendar: Calendar
    private let now: Date
    private let version: String

    init(defaults: UserDefaults, calendar: Calendar = .current,
         now: Date = .now, version: String) {
        self.defaults = defaults
        self.calendar = calendar
        self.now = now
        self.version = version
    }

    /// Counts today as an active day, once per calendar day.
    func recordLaunch() {
        if defaults.object(forKey: Preferences.Key.reviewFirstLaunch) == nil {
            defaults.set(now, forKey: Preferences.Key.reviewFirstLaunch)
        }
        if let lastDay = defaults.object(forKey: Preferences.Key.reviewLastActiveDay) as? Date,
           calendar.isDate(lastDay, inSameDayAs: now) {
            return
        }
        let activeDays = defaults.integer(forKey: Preferences.Key.reviewActiveDays)
        defaults.set(activeDays + 1, forKey: Preferences.Key.reviewActiveDays)
        defaults.set(now, forKey: Preferences.Key.reviewLastActiveDay)
    }

    /// Counts a cell edit, a successful save, or a successful export.
    func recordEngagement() {
        let engagements = defaults.integer(forKey: Preferences.Key.reviewEngagements)
        defaults.set(min(engagements + 1, 100), forKey: Preferences.Key.reviewEngagements)
    }

    /// Whether the app may ask the system for a review now.
    var canPrompt: Bool {
        guard let firstLaunch = defaults.object(forKey: Preferences.Key.reviewFirstLaunch) as? Date,
              let earliest = calendar.date(byAdding: .day, value: 7, to: firstLaunch),
              now >= earliest else { return false }
        guard defaults.integer(forKey: Preferences.Key.reviewActiveDays) >= 3,
              defaults.integer(forKey: Preferences.Key.reviewEngagements) >= 5 else { return false }
        guard defaults.string(forKey: Preferences.Key.reviewLastPromptedVersion) != version else {
            return false
        }
        if let lastPrompt = defaults.object(forKey: Preferences.Key.reviewLastPromptedAt) as? Date,
           let next = calendar.date(byAdding: .day, value: 90, to: lastPrompt),
           now < next {
            return false
        }
        return true
    }

    /// Claims the one prompt this version may show.
    ///
    /// Returns `false` when the policy is not satisfied. The claim is recorded
    /// before the caller posts the notification, so a suppressed system sheet
    /// cannot turn into a loop.
    @discardableResult
    func claimPrompt() -> Bool {
        guard canPrompt else { return false }
        markPrompted()
        return true
    }

    /// Records an explicit Rate action or a claimed automatic prompt.
    func markPrompted() {
        defaults.set(version, forKey: Preferences.Key.reviewLastPromptedVersion)
        defaults.set(now, forKey: Preferences.Key.reviewLastPromptedAt)
    }
}

/// The app-facing review prompt facade.
///
/// Views observe `requestedNotification` and call the system `requestReview`
/// action. The app never shows its own rating screen.
@MainActor
enum ReviewPrompt {
    static let requestedNotification = Notification.Name("ReviewPrompt.requested")

    /// Opens the App Store review page directly. Used by the explicit Rate
    /// action, which always works even when the system prompt quota is spent.
    static let writeReviewURL = URL(string: "https://apps.apple.com/app/id6827919069?action=write-review")!

    /// The public product page, used for sharing.
    static let shareURL = URL(string: "https://apps.apple.com/app/tabtool-csv-editor/id6827919069")!

    static func currentVersion(bundle: Bundle = .main) -> String {
        bundle.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0.1"
    }

    static func recordLaunch(now: Date = .now, defaults: UserDefaults = .standard,
                             calendar: Calendar = .current) {
        policy(now: now, defaults: defaults, calendar: calendar).recordLaunch()
    }

    static func recordEngagement(now: Date = .now, defaults: UserDefaults = .standard,
                                 calendar: Calendar = .current) {
        let policy = policy(now: now, defaults: defaults, calendar: calendar)
        policy.recordEngagement()
        guard policy.claimPrompt() else { return }
        NotificationCenter.default.post(name: requestedNotification, object: nil)
    }

    static func markPrompted(now: Date = .now, defaults: UserDefaults = .standard,
                             calendar: Calendar = .current) {
        policy(now: now, defaults: defaults, calendar: calendar).markPrompted()
    }

    private static func policy(now: Date, defaults: UserDefaults,
                               calendar: Calendar) -> ReviewPromptPolicy {
        ReviewPromptPolicy(defaults: defaults, calendar: calendar, now: now,
                           version: currentVersion())
    }
}
