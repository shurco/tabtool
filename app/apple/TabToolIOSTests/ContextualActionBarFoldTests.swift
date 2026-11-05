import CoreGraphics
import Testing
@testable import TabTool

@Suite struct FoldGapTests {
    @Test func splitsActionsAroundACenteredFold() {
        let gap = FoldGap.make(fold: CGRect(x: 460, y: 0, width: 32, height: 44),
                               totalWidth: 951,
                               actionCount: 7)
        #expect(gap?.split == 4)
        #expect(gap?.leading == 460)
        #expect(gap?.width == 32)
        #expect(gap?.trailing == 459)
    }

    @Test func roundsTheSplitTowardsTheLeadingSide() {
        let gap = FoldGap.make(fold: CGRect(x: 200, y: 0, width: 20, height: 44),
                               totalWidth: 600,
                               actionCount: 3)
        #expect(gap?.split == 2)
    }

    @Test func ignoresAFoldTouchingAnEdge() {
        #expect(FoldGap.make(fold: CGRect(x: 0, y: 0, width: 20, height: 44),
                             totalWidth: 600, actionCount: 7) == nil)
        #expect(FoldGap.make(fold: CGRect(x: 590, y: 0, width: 20, height: 44),
                             totalWidth: 600, actionCount: 7) == nil)
    }

    @Test func ignoresAnInactiveFold() {
        #expect(FoldGap.make(fold: CGRect(x: 290, y: 0, width: 0, height: 44),
                             totalWidth: 600, actionCount: 7) == nil)
    }

    @Test func ignoresASingleAction() {
        #expect(FoldGap.make(fold: CGRect(x: 290, y: 0, width: 20, height: 44),
                             totalWidth: 600, actionCount: 1) == nil)
    }
}
