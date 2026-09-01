# MSL 1.0.260722032 static analysis

- Unity: `6000.3.12f1`
- IL2CPP metadata: `v39`
- ARM64 native binary: `libil2cpp.so`
- Type definitions: `25,134`
- Methods: `188,146`
- Fields: `115,768`
- Packet schemas: `795` (`353` request, `442` response/message-response)
- Request envelope tags: `346`
- Response envelope tags: `401`

## Envelope tags

The generated global `Request` and `Response` protobuf messages are multiplexing envelopes. Their `*FieldNumber` constants expose protocol payload tags directly.

| Tag | Request | Response |
|---:|---|---|
| 50 | `ReqUserLogin` | `RspUserLogin` |
| 51 | `ReqCheckEnergyChargeTime` | `RspCheckEnergyChargeTime` |
| 60 | `ReqBattleStart` | `RspBattleStart` |
| 61 | `ReqBattleEnd` | `RspBattleEnd` |
| 62 | `ReqMonsterCapture` | `RspMonsterCapture` |
| 63 | `ReqBattleContinue` | `RspBattleContinue` |
| 80 | `ReqShopPurchase` | `RspShopPurchase` |
| 117 | `ReqServerTime` | `RspServerTime` |
| 130 | `ReqRecommendedFriends` | `RspRecommendedFriends` |
| 143 | `ReqWriteChatMessage` | `RspWriteChatMessage` |
| 500 | `ReqCoupon` | `RspCoupon` |
| 1000 | `ReqClanCreate` | `RspClanCreate` |
| 3004 | `ReqBattleSkip` | `RspUserShopMetaUpdate` |
| 3007 | `ReqResetBattleSkipCount` | `RspUserEventDataUpdate` |
| 4000 | `` | `RspChangeSeasonDataType` |

## Selected message schemas

### `ReqUserLogin`

| # | Field | Resolved direct type | TypeIndex |
|---:|---|---|---:|
| 1 | `uuid` | `System.String` | 50836 |
| 2 | `privateKey` | `System.String` | 50836 |
| 3 | `platformType` | `AccountPlatformType` | 31095 |
| 4 | `platformUserId` | `System.String` | 50836 |
| 5 | `clientDeviceInfo` | `MsgClientDeviceInfo` | 43887 |
| 7 | `pushAlarm` | `AccountPushSetting` | 31100 |
| 8 | `os` | `AccountOSType` | 31092 |
| 9 | `pushToken` | `System.String` | 50836 |
| 10 | `fbJwtToken` | `System.String` | 50836 |

### `ReqBattleStart`

| # | Field | Resolved direct type | TypeIndex |
|---:|---|---|---:|
| 1 | `battleType` | `BattleType` | 31944 |
| 2 | `scenario` | `MsgBattleStartScenario` | 43759 |
| 3 | `dungeon` | `MsgBattleStartDungeon` | 43745 |
| 4 | `friendDungeon` | `MsgBattleStartFriendDungeon` | 43749 |
| 5 | `arena` | `MsgBattleStartArena` | 43739 |
| 6 | `infinite` | `MsgBattleStartInfinite` | 43755 |
| 7 | `clan` | `MsgBattleStartClan` | 43743 |
| 8 | `colossus` | `MsgBattleStartColossus` | 43741 |
| 9 | `cvc` | `MsgBattleStartCVC` | 43737 |
| 10 | `lupinDungeon` | `MsgBattleStartLupinDungeon` | 43757 |
| 11 | `champions` | `MsgBattleStartChampions` | 43747 |
| 12 | `worldboss` | `MsgBattleStartWorldBoss` | 43765 |
| 13 | `heroGuardianDungeon` | `MsgBattleStartHeroGuardianDungeon` | 43751 |
| 14 | `subjugation` | `MsgBattleStartSubjugation` | 43761 |
| 15 | `cvd` | `MsgBattleStartCVD` | 43735 |
| 16 | `void` | `MsgBattleStartVoid` | 43763 |
| 17 | `hugeBossDungeon` | `MsgBattleStartHugeBossDungeon` | 43753 |

### `RspBattleEnd`

| # | Field | Resolved direct type | TypeIndex |
|---:|---|---|---:|
| 1 | `battleType` | `BattleType` | 31944 |
| 2 | `applyEventList` | `` | 25460 |
| 3 | `scenario` | `MsgRspBattleEndScenario` | 44291 |
| 4 | `dungeon` | `MsgRspBattleEndDungeon` | 44277 |
| 5 | `friendDungeon` | `MsgRspBattleEndFriendDungeon` | 44281 |
| 6 | `arena` | `MsgRspBattleEndArena` | 44271 |
| 7 | `champions` | `MsgRspBattleEndChampions` | 44275 |
| 8 | `infinite` | `MsgRspBattleEndInfinite` | 44287 |
| 9 | `clan` | `MsgRspBattleEndClan` | 44273 |
| 10 | `cvc` | `MsgRspBattleEndCVC` | 44269 |
| 11 | `lupinDungeon` | `MsgRspBattleEndLupinDungeon` | 44289 |
| 12 | `worldboss` | `MsgRspBattleEndWorldBoss` | 44297 |
| 13 | `applyBoosterList` | `` | 25458 |
| 14 | `heroGuardianDungeon` | `MsgRspBattleEndHeroGuardianDungeon` | 44283 |
| 15 | `subjugation` | `MsgRspBattleEndSubjugation` | 44293 |
| 16 | `cvd` | `MsgRspBattleEndCVD` | 44267 |
| 17 | `void` | `MsgRspBattleEndVoid` | 44295 |
| 18 | `hugebossDungeon` | `MsgRspBattleEndHugeBoss` | 44285 |

### `RspServerTime`

| # | Field | Resolved direct type | TypeIndex |
|---:|---|---|---:|
| 1 | `now` | `System.Int64` | 41504 |
| 2 | `utcNow` | `System.Int64` | 41504 |
| 3 | `timeGap` | `System.Int32` | 41471 |

### `ReqChatMessage`

| # | Field | Resolved direct type | TypeIndex |
|---:|---|---|---:|
| 1 | `domain` | `System.String` | 50836 |
| 2 | `channel` | `System.String` | 50836 |
| 3 | `message` | `System.String` | 50836 |

### `RspChatMessage`

| # | Field | Resolved direct type | TypeIndex |
|---:|---|---|---:|
| 1 | `message` | `System.String` | 50836 |

## Network model

The application contains generated Google.Protobuf messages and a `Clover` packet registry layer (`ReqPacketInfo`, `RspPacketInfo`, `PacketHandlerInfo`, `SentPacketInfo`). The static catalog here recovers message fields and envelope tags. Runtime sequencing, authentication state, and transport framing are intentionally not replayed by the tool.

## Caveats

- The direct `TypeIndex` resolver is build-specific. Closed generic instances such as repeated/map fields are retained as raw indices until native registration data is used.
- A packet class can exist without being a top-level envelope payload. This is why schema counts are larger than envelope-tag counts.
- Regenerate on every game update; do not assume tags or message layouts stay fixed.
