import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as QuickSettings from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'io.github.evokelektrique.AetherGtk.Control';
const OBJECT_PATH = '/io/github/evokelektrique/AetherGtk/Control';
const INTERFACE = 'io.github.evokelektrique.AetherGtk.Control';

const AetherToggle = GObject.registerClass(
class AetherToggle extends QuickSettings.QuickMenuToggle {
    constructor() {
        super({
            title: 'Aether',
            subtitle: 'Disconnected',
            iconName: 'network-vpn-symbolic',
            toggleMode: true,
        });

        this.menu.setHeader('network-vpn-symbolic', 'Aether', 'Disconnected');
        this._profileItem = new PopupMenu.PopupMenuItem('Profile: unavailable', {
            reactive: false,
            can_focus: false,
        });
        this.menu.addMenuItem(this._profileItem);
        this._proxyItem = new PopupMenu.PopupMenuItem('SOCKS5: unavailable', {
            reactive: false,
            can_focus: false,
        });
        this.menu.addMenuItem(this._proxyItem);
        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this.menu.addAction('Open Aether', () => this._call('ShowWindow'));
        this.menu.addAction('Quit Aether', () => this._call('Quit'));

        this._updating = false;
        this._available = false;
        this._status = 'Disconnected';
        this._profile = 'unavailable';
        this._proxyAddress = '';
        this._checkedId = this.connect('notify::checked', () => {
            if (this._updating)
                return;
            this._call(this.checked ? 'Connect' : 'Disconnect');
        });
        this._poll();
        this._pollSource = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
            this._poll();
            return GLib.SOURCE_CONTINUE;
        });
    }

    _call(method) {
        Gio.DBus.session.call(
            BUS_NAME,
            OBJECT_PATH,
            INTERFACE,
            method,
            null,
            null,
            Gio.DBusCallFlags.NONE,
            5000,
            null,
            (connection, result) => {
                try {
                    connection.call_finish(result);
                    if (method !== 'Quit')
                        this._poll();
                } catch (error) {
                    this._available = false;
                    this._applyState();
                    logError(error, `Aether ${method} request failed`);
                }
            }
        );
    }

    _poll() {
        Gio.DBus.session.call(
            BUS_NAME,
            OBJECT_PATH,
            INTERFACE,
            'GetState',
            null,
            new GLib.VariantType('(sss)'),
            Gio.DBusCallFlags.NO_AUTO_START,
            2000,
            null,
            (connection, result) => {
                try {
                    const [status, profile, proxyAddress] = connection.call_finish(result).deep_unpack();
                    this._available = true;
                    this._status = status;
                    this._profile = profile;
                    this._proxyAddress = proxyAddress;
                } catch {
                    this._available = false;
                    this._status = 'Disconnected';
                    this._profile = 'unavailable';
                    this._proxyAddress = '';
                }
                this._applyState();
            }
        );
    }

    _applyState() {
        const status = this._available ? this._status : 'Aether is not running';
        const active = this._available && ['Connecting', 'Connected', 'Disconnecting'].includes(this._status);
        this._updating = true;
        this.checked = active;
        this.subtitle = status;
        this.menu.setHeader('network-vpn-symbolic', 'Aether', status);
        this._profileItem.label.text = `Profile: ${this._profile}`;
        this._proxyItem.label.text = this._proxyAddress
            ? `SOCKS5: ${this._proxyAddress}`
            : 'SOCKS5: unavailable';
        this._updating = false;
    }

    destroy() {
        if (this._pollSource)
            GLib.Source.remove(this._pollSource);
        this.disconnect(this._checkedId);
        super.destroy();
    }
});

const AetherIndicator = GObject.registerClass(
class AetherIndicator extends QuickSettings.SystemIndicator {
    constructor() {
        super();
        this.quickSettingsItems.push(new AetherToggle());
    }

    destroy() {
        this.quickSettingsItems.forEach(item => item.destroy());
        super.destroy();
    }
});

export default class AetherExtension extends Extension {
    enable() {
        this._indicator = new AetherIndicator();
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}
