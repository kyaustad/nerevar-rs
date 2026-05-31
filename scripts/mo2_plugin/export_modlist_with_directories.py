from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import QFileDialog, QMessageBox
import mobase
import csv
import os


class ExportModlistCSV(mobase.IPluginTool):

    def __init__(self):
        super().__init__()
        self._organizer = None
        self._parentWidget = None

    def init(self, organizer):
        self._organizer = organizer
        return True

    def name(self):
        return "Export Enabled Mods CSV for Nerevar"

    def author(self):
        return "Nerevar"

    def description(self):
        return "Exports enabled mods and plugins to CSV"

    def version(self):
        return mobase.VersionInfo(1, 0, 0, mobase.ReleaseType.FINAL)

    def settings(self):
        return []

    def displayName(self):
        return "Export Enabled Mods CSV for Nerevar"

    def tooltip(self):
        return "Export enabled mods and plugins"

    def icon(self):
        return QIcon()

    def setParentWidget(self, widget):
        self._parentWidget = widget

    def requirements(self):
        return []

    def isActive(self):
        return True

    def display(self):
        self.export_csv()

    def export_csv(self):
        output_file, _ = QFileDialog.getSaveFileName(
            self._parentWidget,
            "Export CSV",
            "modlist.csv",
            "CSV Files (*.csv)",
        )

        if not output_file:
            return

        mod_list = self._organizer.modList()
        plugin_list = self._organizer.pluginList()
        rows = []

        for mod_name in mod_list.allModsByProfilePriority():
            state = mod_list.state(mod_name)
            if (state & mobase.ModState.ACTIVE) == 0:
                continue

            priority = mod_list.priority(mod_name)

            try:
                mod_path = mod_list.getMod(mod_name).absolutePath()
                directory_name = os.path.basename(str(mod_path))
            except Exception:
                directory_name = mod_name

            plugins_for_mod = []

            for plugin in plugin_list.pluginNames():
                if plugin_list.origin(plugin) != mod_name:
                    continue

                plugin_state = plugin_list.state(plugin)
                load_order = plugin_list.loadOrder(plugin)

                plugins_for_mod.append({
                    "name": plugin,
                    "enabled": plugin_state == mobase.PluginState.ACTIVE,
                    "load_order": load_order if load_order >= 0 else "",
                })

            if not plugins_for_mod:
                rows.append([directory_name, priority, "", "", ""])
            else:
                for plugin in plugins_for_mod:
                    rows.append([
                        directory_name,
                        priority,
                        plugin["name"],
                        plugin["enabled"],
                        plugin["load_order"],
                    ])

        rows.sort(key=lambda r: (r[1], r[2]))

        with open(output_file, "w", newline="", encoding="utf-8") as csvfile:
            writer = csv.writer(csvfile)
            writer.writerow([
                "Mod Directory",
                "Mod Priority",
                "Plugin Name",
                "Plugin Enabled",
                "Plugin Load Order",
            ])
            writer.writerows(rows)

        QMessageBox.information(
            self._parentWidget,
            "Export Complete",
            f"Exported {len(rows)} entries to:\n{output_file}",
        )


def createPlugin():
    return ExportModlistCSV()
