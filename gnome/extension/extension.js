// SPDX-License-Identifier: GPL-3.0-or-later

import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const POLL_INTERVAL_MS = 750;
const MAX_PREVIEW = 80;
const HISTORY_FILE = 'history.json';
const CONFIG_FILE = 'config.json';

const DEFAULT_CONFIG = {
    confirmBeforeClearAll: true,
    maxEntries: 100,
    maxAgeDays: 30,
    keepPinnedOnClearUnpinned: true,
    privateMode: false,
    uniqueSession: false,
    sensitiveFilter: true,
    maxTextBytes: 1024 * 1024,
};

const SENSITIVE_PATTERNS = [
    /-----BEGIN [A-Z ]*PRIVATE KEY-----/i,
    /\bAKIA[0-9A-Z]{16}\b/,
    /\bgh[pousr]_[A-Za-z0-9_]{30,}\b/,
    /\b(?:password|passwd|pwd|secret|token|api[_-]?key)\s*[:=]\s*\S+/i,
];

function nowSeconds() {
    return Math.floor(Date.now() / 1000);
}

function textByteLength(text) {
    return new TextEncoder().encode(text).length;
}

function looksSensitive(text) {
    return SENSITIVE_PATTERNS.some(pattern => pattern.test(text));
}

function previewText(text) {
    return text.replace(/\s+/g, ' ').slice(0, MAX_PREVIEW);
}

function clampNumber(value, fallback, min, max) {
    if (!Number.isFinite(value)) {
        return fallback;
    }

    return Math.max(min, Math.min(max, Math.floor(value)));
}

const ClipboardIndicator = GObject.registerClass(
class ClipboardIndicator extends PanelMenu.Button {
    _init(extension) {
        super._init(0.0, 'Tihulu Clipboard Manager');

        this._extension = extension;
        this._clipboard = St.Clipboard.get_default();
        this._history = [];
        this._config = {...DEFAULT_CONFIG};
        this._lastSeen = null;
        this._query = '';
        this._confirmClearAll = false;
        this._timeoutId = 0;

        const icon = new St.Icon({
            icon_name: 'edit-paste-symbolic',
            style_class: 'system-status-icon',
        });
        this.add_child(icon);

        this._loadConfig();
        if (this._config.uniqueSession) {
            this._history = [];
            this._saveHistory();
        } else {
            this._loadHistory();
        }
        this._pruneHistory();
        this._buildMenu();
        this._startWatcher();
    }

    destroy() {
        if (this._timeoutId) {
            GLib.source_remove(this._timeoutId);
            this._timeoutId = 0;
        }

        this._saveConfig();
        this._saveHistory();
        super.destroy();
    }

    _dataDir() {
        const dir = GLib.build_filenamev([
            GLib.get_user_data_dir(),
            'tihulu-clipboard-manager-gnome',
        ]);
        GLib.mkdir_with_parents(dir, 0o700);
        return dir;
    }

    _historyPath() {
        return GLib.build_filenamev([this._dataDir(), HISTORY_FILE]);
    }

    _configPath() {
        return GLib.build_filenamev([this._dataDir(), CONFIG_FILE]);
    }

    _readJson(path, fallback) {
        try {
            const [ok, bytes] = GLib.file_get_contents(path);
            if (!ok) {
                return fallback;
            }

            const decoder = new TextDecoder('utf-8');
            return JSON.parse(decoder.decode(bytes));
        } catch (error) {
            logError(error, `Failed to read ${path}`);
            return fallback;
        }
    }

    _writeJson(path, value) {
        try {
            GLib.file_set_contents(path, JSON.stringify(value, null, 2));
            GLib.chmod(path, 0o600);
        } catch (error) {
            logError(error, `Failed to write ${path}`);
        }
    }

    _loadConfig() {
        const loaded = this._readJson(this._configPath(), {});
        this._config = {
            ...DEFAULT_CONFIG,
            ...loaded,
        };
        this._config.maxEntries = clampNumber(this._config.maxEntries, DEFAULT_CONFIG.maxEntries, 1, 500);
        this._config.maxAgeDays = clampNumber(this._config.maxAgeDays, DEFAULT_CONFIG.maxAgeDays, 0, 3650);
        this._config.maxTextBytes = clampNumber(this._config.maxTextBytes, DEFAULT_CONFIG.maxTextBytes, 1, 10 * 1024 * 1024);
    }

    _saveConfig() {
        this._writeJson(this._configPath(), this._config);
    }

    _loadHistory() {
        const parsed = this._readJson(this._historyPath(), []);
        if (!Array.isArray(parsed)) {
            this._history = [];
            return;
        }

        this._history = parsed
            .map((item, index) => this._normalizeEntry(item, index))
            .filter(entry => entry !== null);
    }

    _normalizeEntry(item, index) {
        if (typeof item === 'string') {
            return {
                id: `${Date.now()}-${index}`,
                type: 'text',
                text: item,
                pinned: false,
                createdAt: nowSeconds(),
            };
        }

        if (!item || item.type !== 'text' || typeof item.text !== 'string') {
            return null;
        }

        return {
            id: typeof item.id === 'string' ? item.id : `${Date.now()}-${index}`,
            type: 'text',
            text: item.text,
            pinned: item.pinned === true,
            createdAt: Number.isFinite(item.createdAt) ? item.createdAt : nowSeconds(),
        };
    }

    _saveHistory() {
        this._writeJson(this._historyPath(), this._history);
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
        if (!text || !text.trim() || this._config.privateMode) {
            return;
        }

        if (text === this._lastSeen) {
            return;
        }

        if (textByteLength(text) > this._config.maxTextBytes) {
            return;
        }

        if (this._config.sensitiveFilter && looksSensitive(text)) {
            return;
        }

        this._lastSeen = text;
        this._history = this._history.filter(entry => entry.text !== text);
        this._history.unshift({
            id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
            type: 'text',
            text,
            pinned: false,
            createdAt: nowSeconds(),
        });
        this._pruneHistory();
        this._saveHistory();
        this._buildMenu();
    }

    _pruneHistory() {
        if (this._config.maxAgeDays > 0) {
            const cutoff = nowSeconds() - this._config.maxAgeDays * 24 * 60 * 60;
            this._history = this._history.filter(entry => entry.pinned || entry.createdAt >= cutoff);
        }

        const pinned = this._history.filter(entry => entry.pinned);
        const unpinned = this._history.filter(entry => !entry.pinned);
        const unpinnedBudget = Math.max(0, this._config.maxEntries - pinned.length);
        this._history = [...pinned, ...unpinned.slice(0, unpinnedBudget)];
    }

    _copyText(text) {
        this._clipboard.set_text(St.ClipboardType.CLIPBOARD, text);
        this._lastSeen = text;
    }

    _deleteEntry(id) {
        this._history = this._history.filter(entry => entry.id !== id);
        this._saveHistory();
        this._rebuildEntriesOnly();
    }

    _togglePin(id) {
        const entry = this._history.find(item => item.id === id);
        if (!entry) {
            return;
        }

        entry.pinned = !entry.pinned;
        this._saveHistory();
        this._rebuildEntriesOnly();
    }

    _clearAll() {
        this._history = [];
        this._lastSeen = null;
        this._confirmClearAll = false;
        this._saveHistory();
        this._buildMenu();
    }

    _clearUnpinned() {
        this._history = this._history.filter(entry => entry.pinned);
        this._saveHistory();
        this._rebuildEntriesOnly();
    }

    _buildMenu() {
        this.menu.removeAll();

        const titleItem = new PopupMenu.PopupMenuItem('Tihulu Clipboard Manager', {
            reactive: false,
        });
        this.menu.addMenuItem(titleItem);

        const status = [
            this._config.sensitiveFilter ? 'Sensitive filter on' : 'Sensitive filter off',
            this._config.privateMode ? 'Private mode on' : 'Private mode off',
            `${this._history.length}/${this._config.maxEntries} entries`,
        ].join(' · ');
        this.menu.addMenuItem(new PopupMenu.PopupMenuItem(status, {reactive: false}));

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

        this._privateItem = new PopupMenu.PopupSwitchMenuItem('Private mode', this._config.privateMode);
        this._privateItem.connect('toggled', (_item, state) => {
            this._config.privateMode = state;
            this._saveConfig();
            this._buildMenu();
        });
        this.menu.addMenuItem(this._privateItem);

        this._sensitiveItem = new PopupMenu.PopupSwitchMenuItem('Sensitive filter', this._config.sensitiveFilter);
        this._sensitiveItem.connect('toggled', (_item, state) => {
            this._config.sensitiveFilter = state;
            this._saveConfig();
            this._buildMenu();
        });
        this.menu.addMenuItem(this._sensitiveItem);

        this._uniqueSessionItem = new PopupMenu.PopupSwitchMenuItem('Unique session', this._config.uniqueSession);
        this._uniqueSessionItem.connect('toggled', (_item, state) => {
            this._config.uniqueSession = state;
            this._saveConfig();
            if (state) {
                this._clearAll();
            }
        });
        this.menu.addMenuItem(this._uniqueSessionItem);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._entriesSection = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._entriesSection);
        this._rebuildEntriesOnly();

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        const clearUnpinnedItem = new PopupMenu.PopupMenuItem('Clear unpinned');
        clearUnpinnedItem.connect('activate', () => this._clearUnpinned());
        this.menu.addMenuItem(clearUnpinnedItem);

        const clearLabel = this._confirmClearAll ? 'Confirm erase all' : 'Erase all';
        const clearItem = new PopupMenu.PopupMenuItem(clearLabel);
        clearItem.connect('activate', () => {
            if (this._config.confirmBeforeClearAll && !this._confirmClearAll) {
                this._confirmClearAll = true;
                this._buildMenu();
                return;
            }

            this._clearAll();
        });
        this.menu.addMenuItem(clearItem);
    }

    _rebuildEntriesOnly() {
        if (!this._entriesSection) {
            return;
        }

        this._entriesSection.removeAll();
        const filtered = this._history.filter(entry => entry.text.toLowerCase().includes(this._query));

        if (filtered.length === 0) {
            this._entriesSection.addMenuItem(new PopupMenu.PopupMenuItem('No clipboard entries', {
                reactive: false,
            }));
            return;
        }

        for (const entry of filtered) {
            const row = new PopupMenu.PopupBaseMenuItem({reactive: false});
            const marker = entry.pinned ? '●' : '○';
            const label = new St.Label({
                text: `${marker} ${previewText(entry.text)}`,
                x_expand: true,
            });
            row.add_child(label);

            row.add_child(this._entryButton('Copy', () => this._copyText(entry.text)));
            row.add_child(this._entryButton(entry.pinned ? 'Unpin' : 'Pin', () => this._togglePin(entry.id)));
            row.add_child(this._entryButton('Delete', () => this._deleteEntry(entry.id)));
            this._entriesSection.addMenuItem(row);
        }
    }

    _entryButton(label, callback) {
        const button = new St.Button({
            label,
            style_class: 'button',
            can_focus: true,
        });
        button.connect('clicked', callback);
        return button;
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
