// SPDX-License-Identifier: GPL-3.0-or-later

import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const POLL_INTERVAL_MS = 900;
const HELPER_NAME = 'tihulu-gnome-clipboard-helper';

function helperPath() {
    return GLib.build_filenamev([GLib.get_home_dir(), '.local', 'bin', HELPER_NAME]);
}

function decodeBytes(bytes) {
    return new TextDecoder('utf-8').decode(bytes);
}

const ClipboardIndicator = GObject.registerClass(
class ClipboardIndicator extends PanelMenu.Button {
    _init(extension) {
        super._init(0.0, 'Tihulu Clipboard Manager');

        this._extension = extension;
        this._entries = [];
        this._config = {};
        this._query = '';
        this._confirmClearAll = false;
        this._timeoutId = 0;
        this._lastError = null;
        this._stateFingerprint = '';

        this.add_child(new St.Icon({
            icon_name: 'edit-paste-symbolic',
            style_class: 'system-status-icon',
        }));

        this._refresh('state');
        this._startWatcher();
    }

    destroy() {
        if (this._timeoutId) {
            GLib.source_remove(this._timeoutId);
            this._timeoutId = 0;
        }

        super.destroy();
    }

    _startWatcher() {
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, POLL_INTERVAL_MS, () => {
            this._refresh('capture');
            return GLib.SOURCE_CONTINUE;
        });
    }

    _runHelper(args) {
        try {
            const [ok, stdout, stderr, status] = GLib.spawn_sync(
                null,
                [helperPath(), ...args],
                null,
                GLib.SpawnFlags.SEARCH_PATH,
                null
            );

            if (!ok || status !== 0) {
                const detail = stderr ? decodeBytes(stderr).trim() : `exit status ${status}`;
                throw new Error(detail || 'helper failed');
            }

            return JSON.parse(stdout ? decodeBytes(stdout).trim() : '{}');
        } catch (error) {
            this._lastError = `${error.message}. Run gnome/scripts/quick-install.sh again.`;
            logError(error, 'Tihulu GNOME helper failed');
            return null;
        }
    }

    _applyState(state) {
        if (!state) {
            return false;
        }

        const entries = Array.isArray(state.entries) ? state.entries : [];
        const config = state.config || {};
        const fingerprint = JSON.stringify({entries, config});
        const changed = fingerprint !== this._stateFingerprint || this._lastError !== null;

        this._lastError = null;
        this._entries = entries;
        this._config = config;
        this._stateFingerprint = fingerprint;

        return changed;
    }

    _refresh(command = 'state') {
        const errorBefore = this._lastError;
        const changed = this._applyState(this._runHelper([command]));

        // The capture timer runs often. Rebuilding the menu while it is open makes
        // GNOME Shell look like the applet is constantly refreshing and also breaks
        // typing in the search field. Keep the current popup stable; new entries
        // will appear on the next manual action or when the popup is reopened.
        if (command === 'capture' && this.menu.isOpen) {
            return;
        }

        if (changed || command !== 'capture' || this._lastError !== errorBefore) {
            this._buildMenu();
        }
    }

    _helperAction(args) {
        this._applyState(this._runHelper(args));
        this._buildMenu();
    }

    _setConfig(key, value) {
        this._helperAction(['set', key, value ? 'true' : 'false']);
    }

    _buildMenu() {
        this.menu.removeAll();
        this.menu.addMenuItem(new PopupMenu.PopupMenuItem('Tihulu Clipboard Manager', {reactive: false}));

        if (this._lastError) {
            this.menu.addMenuItem(new PopupMenu.PopupMenuItem(this._lastError, {reactive: false}));
            return;
        }

        this.menu.addMenuItem(new PopupMenu.PopupMenuItem(this._statusText(), {reactive: false}));
        this._addSearch();
        this._addSwitch('Private mode', this._config.privateMode, value => this._setConfig('privateMode', value));
        this._addSwitch('Unique session', this._config.uniqueSession, value => this._setConfig('uniqueSession', value));
        this._addSwitch('Encrypted history', this._config.encryptHistory, value => this._setConfig('encryptHistory', value));
        this._addSwitch('Sensitive filter', this._config.sensitiveFilter, value => this._setConfig('sensitiveFilter', value));
        this._addSwitch('Image history', this._config.imageClipboard, value => this._setConfig('imageClipboard', value));
        this._addSwitch('Image size limit', this._config.limitImageSize, value => this._setConfig('limitImageSize', value));

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._entriesSection = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._entriesSection);
        this._rebuildEntriesOnly();

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        const clearUnpinnedItem = new PopupMenu.PopupMenuItem('Clear unpinned');
        clearUnpinnedItem.connect('activate', () => this._helperAction(['clear-unpinned']));
        this.menu.addMenuItem(clearUnpinnedItem);

        const clearItem = new PopupMenu.PopupMenuItem(this._confirmClearAll ? 'Confirm erase all' : 'Erase all');
        clearItem.connect('activate', () => {
            if (this._config.confirmBeforeClearAll && !this._confirmClearAll) {
                this._confirmClearAll = true;
                this._buildMenu();
                return;
            }

            this._confirmClearAll = false;
            this._helperAction(['clear-all']);
        });
        this.menu.addMenuItem(clearItem);
    }

    _statusText() {
        return [
            this._config.encryptHistory ? 'Encrypted' : 'Plain JSON',
            this._config.imageClipboard ? 'Images on' : 'Images off',
            this._config.privateMode ? 'Private on' : 'Private off',
            `${this._entries.length}/${this._config.maxEntries ?? '?'} entries`,
        ].join(' · ');
    }

    _addSearch() {
        const searchItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
        const entry = new St.Entry({
            hint_text: 'Search clipboard history',
            text: this._query,
            can_focus: true,
            x_expand: true,
        });
        entry.get_clutter_text().connect('text-changed', () => {
            this._query = entry.get_text().toLowerCase();
            this._rebuildEntriesOnly();
        });
        searchItem.add_child(entry);
        this.menu.addMenuItem(searchItem);
    }

    _addSwitch(label, value, callback) {
        const item = new PopupMenu.PopupSwitchMenuItem(label, value === true);
        item.connect('toggled', (_item, state) => callback(state));
        this.menu.addMenuItem(item);
    }

    _rebuildEntriesOnly() {
        if (!this._entriesSection) {
            return;
        }

        this._entriesSection.removeAll();
        const filtered = this._entries.filter(entry => entry.preview.toLowerCase().includes(this._query));
        if (filtered.length === 0) {
            this._entriesSection.addMenuItem(new PopupMenu.PopupMenuItem('No clipboard entries', {reactive: false}));
            return;
        }

        for (const entry of filtered) {
            const row = new PopupMenu.PopupBaseMenuItem({reactive: false});
            const marker = entry.pinned ? '●' : '○';
            const prefix = entry.kind === 'image' ? 'Image: ' : '';
            row.add_child(new St.Label({text: `${marker} ${prefix}${entry.preview}`, x_expand: true}));
            row.add_child(this._entryButton('Copy', () => this._helperAction(['copy', `${entry.id}`])));
            row.add_child(this._entryButton(entry.pinned ? 'Unpin' : 'Pin', () => this._helperAction(['toggle-pin', `${entry.id}`])));
            row.add_child(this._entryButton('Delete', () => this._helperAction(['delete', `${entry.id}`])));
            this._entriesSection.addMenuItem(row);
        }
    }

    _entryButton(label, callback) {
        const button = new St.Button({label, style_class: 'button', can_focus: true});
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
