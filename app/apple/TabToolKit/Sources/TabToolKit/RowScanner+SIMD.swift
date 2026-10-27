import Foundation

// The fast path classifies 64 bytes at a time into bit masks (one bit per
// byte) and derives row ends from them, so quote state costs a prefix-xor
// instead of a branch per byte. It requires that a quote is escaped by
// doubling (or quoting is off), because a dedicated escape character would
// make the quote state depend on the previous byte.
//
// Mask building follows the shape that measured 13 GB/s: compare 16 lanes,
// bitcast the lane mask to bytes, AND with bit weights, then sum each half.
// Per-lane loops, SIMD64 comparisons and `replacing(with:where:)` compile to
// code that is 50 to 300 times slower.

private let bitWeights = SIMD16<UInt8>(1, 2, 4, 8, 16, 32, 64, 128,
                                       1, 2, 4, 8, 16, 32, 64, 128)

private func mask16(_ vector: SIMD16<UInt8>, _ target: SIMD16<UInt8>) -> UInt64 {
    let bits = unsafeBitCast(vector .== target, to: SIMD16<UInt8>.self) & bitWeights
    return UInt64(bits.lowHalf.wrappedSum()) | UInt64(bits.highHalf.wrappedSum()) << 8
}

private func mask64(_ pointer: UnsafeRawPointer, _ target: SIMD16<UInt8>) -> UInt64 {
    let v0 = pointer.loadUnaligned(as: SIMD16<UInt8>.self)
    let v1 = (pointer + 16).loadUnaligned(as: SIMD16<UInt8>.self)
    let v2 = (pointer + 32).loadUnaligned(as: SIMD16<UInt8>.self)
    let v3 = (pointer + 48).loadUnaligned(as: SIMD16<UInt8>.self)
    return mask16(v0, target) | mask16(v1, target) << 16
        | mask16(v2, target) << 32 | mask16(v3, target) << 48
}

/// Bits `low...high` set, for bit indices 0...63.
private func bitRange(_ low: Int, _ high: Int) -> UInt64 {
    let upper: UInt64 = high >= 63 ? ~0 : (1 << UInt64(high + 1)) &- 1
    let lower: UInt64 = (1 << UInt64(low)) &- 1
    return upper & ~lower
}

extension RowScanner {
    /// Scans one full 64-byte block starting at absolute offset `base`.
    mutating func scanBlock(_ pointer: UnsafeRawPointer, base: Int,
                            into index: inout RowIndex) {
        // An absent quote or separator contributes an empty mask.
        let quoteMask = quote.map { mask64(pointer, SIMD16(repeating: $0)) } ?? 0
        let lineFeeds = mask64(pointer, SIMD16(repeating: Self.lineFeed))
        let carriageReturns = mask64(pointer, SIMD16(repeating: Self.carriageReturn))
        let separatorMask = separator.map { mask64(pointer, SIMD16(repeating: $0)) } ?? 0

        var parity = quoteMask
        parity ^= parity << 1
        parity ^= parity << 2
        parity ^= parity << 4
        parity ^= parity << 8
        parity ^= parity << 16
        parity ^= parity << 32
        let inside = parity ^ (inQuote ? ~0 : 0)

        let outsideLF = lineFeeds & ~inside
        let outsideCR = carriageReturns & ~inside
        let carry: UInt64 = afterCarriageReturn ? 1 : 0
        let swallowed = outsideLF & ((outsideCR << 1) | carry)
        let separatorsOutside = separatorMask & ~inside

        var rowEnds = (outsideLF | outsideCR) & ~swallowed
        var segmentStart = 0
        while segmentStart < Self.blockBytes {
            let terminated = rowEnds != 0
            let last = terminated ? rowEnds.trailingZeroBitCount : Self.blockBytes - 1
            rowEnds &= rowEnds &- 1

            var first = segmentStart
            if atRowStart {
                if (swallowed >> UInt64(first)) & 1 == 1 {
                    rowStart = base + first + 1
                    first += 1
                }
                if first <= last { beginRow(into: &index) }
            }
            if first <= last {
                separators += (separatorsOutside & bitRange(first, last)).nonzeroBitCount
            }
            if terminated { endRow(at: base + last + 1, into: &index) }
            segmentStart = last + 1
        }

        afterCarriageReturn = (outsideCR >> 63) & 1 == 1
        inQuote = (inside >> 63) & 1 == 1
    }
}
