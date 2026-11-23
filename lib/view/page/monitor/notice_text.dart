import 'package:server_box/core/extension/context/locale.dart';
import 'package:server_box/data/model/server/monitor/monitor_overview.dart';

/// [notice] in this app's language, from its `kind` and `params`; the agent's
/// own English for a kind this app does not know and for rows from before
/// kinds.
(String subject, String body) noticeText(MonitorNotice notice) {
  int count(String key) => int.tryParse(notice.param(key)) ?? 0;
  return switch (notice.kind) {
    'agent.update' => (
      l10n.noticeAgentUpdate,
      l10n.monitorAgentOutdatedBody(notice.param('version'), notice.param('current')),
    ),
    'storage.arrayDegraded' => (
      l10n.noticeStorageArrayDegraded(notice.param('array')),
      l10n.noticeStorageArrayDegradedBody,
    ),
    'storage.arrayBack' => (l10n.noticeStorageArrayBack(notice.param('array')), l10n.noticeStorageArrayBackBody),
    'storage.poolUnhealthy' => (
      l10n.noticeStoragePoolUnhealthy(notice.param('pool'), notice.param('health')),
      l10n.noticeStoragePoolUnhealthyBody,
    ),
    'storage.poolBack' => (l10n.noticeStoragePoolBack(notice.param('pool')), l10n.noticeStoragePoolBackBody),
    'storage.diskFailing' => (
      l10n.noticeStorageDiskFailing(notice.param('device')),
      l10n.noticeStorageDiskFailingBody,
    ),
    'storage.diskSectors' => (
      l10n.noticeStorageDiskSectors(notice.param('device'), count('count')),
      l10n.noticeStorageDiskSectorsBody(count('was')),
    ),
    'storage.maintenanceFailed' => (
      l10n.noticeStorageMaintenanceFailed(switch (notice.param('task')) {
        'scrub' => l10n.storageTaskScrub,
        'smartShort' => l10n.storageTaskSmartShort,
        'smartLong' => l10n.storageTaskSmartLong,
        final task => task,
      }),
      switch (notice.param('error')) {
        'sudoNeedsPassword' => l10n.storageNeedsSudo,
        final error => error,
      },
    ),
    _ => (notice.subject, notice.body),
  };
}
