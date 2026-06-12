// SPDX-License-Identifier: GPL-3.0-or-later

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const POLL_INTERVAL_MS = 750;
const MAX_ENTRIES = 50;
const MAX_PREVIEW = 80;
const HISTORY_FILE = 'history.json';

const ClipboardIndicator = GObject.registerClass(
class ClipboardIndicator extends PanelMenu.Button {
    _init(extension) {
        super._init(0.0, 'Tihulu Clipboard Manager');

        this._extension = extension;
        this._clipboard = St.Clipboard.get_default();
        this._history = [];
        this._lastSeen = null;
        this._query = '';
        this._privateMode = false;
        this._timeoutId = 0;

        const icon = new St.Icon({
            icon_name: 'edit-paste-symbolic',
            style_class: 'system-status-icon',
        });
        this.add_child(icon);

        this._loadHistory();
        this._buildMenu();
        this._startWatcher();
    }

    destroy() {
        if (this._timeoutId) {
            GLib.source_remove(this._timeoutId);
            this._timeoutId = 0;
        }

        this._saveHistory();
        super.destroy();
    }

    _historyPath() {
        const dir = GLib.build_filenamev([
            GLib.get_user_data_dir(),
            'tihulu-clipboard-manager-gnome',
        ]);
        GLib.mkdir_with_parents(dir, 0o700);
        return GLib.build_filenamev([dir, HISTORY_FILE]);
    }

    _loadHistory() {
        try {
            const [ok, bytes] = GLib.file_get_contents(this._historyPath());
            if (!ok) {
                return;
            }

            const decoder = new TextDecoder('utf-8');
            const parsed = JSON.parse(decoder.decode(bytes));
            if (Array.isArray(parsed)) {
                this._history = parsed.filter(item => typeof item === 'string');
            }
        } catch (error) {
            logError(error, 'Failed to load Tihulu GNOME clipboard history');
            this._history = [];
        }
    }

    _saveHistory() {
        try {
            const payload = JSON.stringify(this._history, null, 2);
            GLib.file_set_contents(this._historyPath(), payload);
        } catch (error) {
            logError(error, 'Failed to save Tihulu GNOME clipboard history');
        }
    }

    _startWatcher() {
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, POLL_INTERVAL_MS, () => {
            this._clipboard.get_text(St.ClipboardType.CLIPBOARD, (_clipboard, text) => {
                this._handleClipboardText(text);
            });
            return GLib.SOURCE_CONTINUE;
        });
    }

    _handleClipboardText(text) {
        if (!text || !text.trim() || this._privateMode) {
            return;
        }

        if (text === this._lastSeen) {
            return;
        }

        this._lastSeen = text;
        this._history = this._history.filter(item => item !== text);
        this._history.unshift(text);
        this._history = this._history.slice(0, MAX_ENTRIES);
        this._saveHistory();
        this._buildMenu();
    }

    _copyText(text) {
        this._clipboard.set_text(St.ClipboardType.CLIPBOARD, text);
        this._lastSeen = text;
    }

    _buildMenu() {
        this.menu.removeAll();

        const titleItem = new PopupMenu.PopupMenuItem('Tihulu Clipboard Manager', {
            reactive: false,
        });
        this.menu.addMenuItem(titleItem);

        const searchItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
        this._searchEntry = new St.Entry({
            hint_text: 'Search clipboard history',
            text: this._query,
            can_focus: true,
            x_expand: true,
        });
        const clutterText = this._searchEntry.get_clutter_text();
        clutterText.connect('text-changed', () => {
            this._query = this._searchEntry.get_text().toLowerCase();
            this._rebuildEntriesOnly();
        });
        searchItem.add_child(this._searchEntry);
        this.menu.addMenuItem(searchItem);

        this._privateItem = new PopupMenu.PopupSwitchMenuItem('Private mode', this._privateMode);
        this._privateItem.connect('toggled', (_item, state) => {
            this._privateMode = state;
        });
        this.menu.addMenuItem(this._privateItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._entriesSection = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._entriesSection);
        this._rebuildEntriesOnly();

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        const clearItem = new PopupMenu.PopupMenuItem('Clear history');
        clearItem.connect('activate', () => {
            this._history = [];
            this._lastSeen = null;
            this._saveHistory();
            this._rebuildEntriesOnly();
        });
        this.menu.addMenuItem(clearItem);
    }

    _rebuildEntriesOnly() {
        if (!this._entriesSection) {
            return;
        }

        this._entriesSection.removeAll();
        const filtered = this._history.filter(text => text.toLowerCase().includes(this._query));

        if (filtered.length === 0) {
            this._entriesSection.addMenuItem(new PopupMenu.PopupMenuItem('No clipboard entries', {
                reactive: false,
            }));
            return;
        }

        for (const text of filtered) {
            const preview = text.replace(/\s+/g, ' ').slice(0, MAX_PREVIEW);
            const item = new PopupMenu.PopupMenuItem(preview);
            item.connect('activate', () => this._copyText(text));
            this._entriesSection.addMenuItem(item);
        }
    }
});

export default class TihuluClipboardManagerExtension extends Extension {
    enable() {
        this._indicator = new ClipboardIndicator(this);
        Main.panel.addToStatusArea(this.uuid, this._indicator);
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}
