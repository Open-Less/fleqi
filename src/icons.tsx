import type { CSSProperties } from 'react';
import send from './assets/icons/common/paperplane.fill.svg';
import brain from './assets/icons/common/brain.filled.head.profile.svg';
import folder from './assets/icons/folder/folder.svg';
import trash from './assets/icons/trash/trash.svg';
import sparkles from './assets/icons/common/sparkles.2.svg';
import arrowUpMessage from './assets/icons/common/arrow.up.message.svg';
import checkmarkMessage from './assets/icons/common/checkmark.message.svg';
import commandSquare from './assets/icons/common/command.square.fill.svg';
import lassoSparkles from './assets/icons/common/lasso.badge.sparkles.svg';
import linkBadgePlus from './assets/icons/common/link.badge.plus.svg';
import paperplaneCircle from './assets/icons/common/paperplane.circle.fill.svg';
import piGlyph from './assets/icons/common/pi.svg';
import plusCapsuleFill from './assets/icons/common/plus.capsule.fill.svg';
import plusCapsule from './assets/icons/common/plus.capsule.svg';
import pointerRays from './assets/icons/common/pointer.arrow.ipad.rays.svg';
import pointerSlashSquare from './assets/icons/common/pointer.arrow.ipad.slash.square.fill.svg';
import pointerSlash from './assets/icons/common/pointer.arrow.ipad.slash.svg';
import pointerSquare from './assets/icons/common/pointer.arrow.ipad.square.fill.svg';
import pointer from './assets/icons/common/pointer.arrow.ipad.svg';
import textSpacing from './assets/icons/common/text.word.spacing.svg';
import widgetPlus from './assets/icons/common/widget.extralarge.badge.plus.svg';
import folderFill from './assets/icons/folder/folder.fill.svg';
import folderBadgePlus from './assets/icons/folder/folder.badge.plus.svg';
import folderFillBadgePlus from './assets/icons/folder/folder.fill.badge.plus.svg';
import accessibility from './assets/icons/common/accessibility.svg';
import internalDrive from './assets/icons/common/internaldrive.svg';
import internalDriveFill from './assets/icons/common/internaldrive.fill.svg';
import folderGear from './assets/icons/common/folder.badge.gearshape.svg';
import gearshape2 from './assets/icons/common/gearshape.2.svg';
import paintpalette from './assets/icons/common/paintpalette.svg';
import customLink from './assets/icons/common/custom.link.svg';
import shield from './assets/icons/common/shield.lefthalf.filled.svg';
import shieldCheckmark from './assets/icons/common/shield.lefthalf.filled.badge.checkmark.svg';
import shieldSlash from './assets/icons/common/shield.lefthalf.filled.slash.svg';
import shieldExclamation from './assets/icons/common/shield.lefthalf.filled.trianglebadge.exclamationmark.svg';
import clockRotate from './assets/icons/common/clock.arrow.trianglehead.counterclockwise.rotate.90.svg';
import infoCircle from './assets/icons/common/info.circle.svg';
import plusCircleFill from './assets/icons/common/plus.circle.fill.svg';
import chevronDown from './assets/icons/common/chevron.down.svg';
import chevronUp from './assets/icons/common/chevron.up.svg';
import magnifyingglass from './assets/icons/common/magnifyingglass.svg';
import docOnDocument from './assets/icons/common/document.on.document.svg';
import checkmarkCircleFill from './assets/icons/common/checkmark.circle.fill.svg';
import exclamationTriangle from './assets/icons/common/exclamationmark.triangle.fill.svg';
import circleHalf from './assets/icons/common/circle.lefthalf.filled.svg';
import sliderHalf from './assets/icons/common/slider.horizontal.below.circle.lefthalf.filled.svg';
import sparkleMagnifyingglass from './assets/icons/common/sparkle.magnifyingglass.svg';
import eraserFill from './assets/icons/common/eraser.fill.svg';
const glyphs={send,brain,folder,trash,sparkles,arrowUpMessage,checkmarkMessage,commandSquare,lassoSparkles,linkBadgePlus,paperplaneCircle,piGlyph,plusCapsuleFill,plusCapsule,pointerRays,pointerSlashSquare,pointerSlash,pointerSquare,pointer,textSpacing,widgetPlus,folderFill,folderBadgePlus,folderFillBadgePlus,accessibility,internalDrive,internalDriveFill,folderGear,gearshape2,paintpalette,customLink,shield,shieldCheckmark,shieldSlash,shieldExclamation,clockRotate,infoCircle,plusCircleFill,chevronDown,chevronUp,magnifyingglass,docOnDocument,checkmarkCircleFill,exclamationTriangle,circleHalf,sliderHalf,sparkleMagnifyingglass,eraserFill};
export type GlyphName=keyof typeof glyphs;
export function AssetIcon({name,className='',size=18}:{name:GlyphName;className?:string;size?:number}) {
  return <span aria-hidden="true" className={`asset-icon ${className}`} style={{width:size,height:size,'--icon-url':`url("${glyphs[name]}")`} as CSSProperties}/>;
}
// 带品牌色的图标：跟随明暗主题切换深浅两套颜色（取自应用图标配色）。
export function TintIcon({name,size=16,light,dark}:{name:GlyphName;size?:number;light:string;dark:string}) {
  return <span aria-hidden="true" className="tint-icon" style={{width:size,height:size,'--tint-light':light,'--tint-dark':dark,'--icon-url':`url("${glyphs[name]}")`} as CSSProperties}/>;
}
// Apple 导出的 fill 系列图标是“彩色圆底 + 白色图案”，不能用 alpha mask
// 单色化（会变成实心色块），必须按原始设计以 <img> 呈现。
export function SvgImage({name,size=20,className=''}:{name:GlyphName;size?:number;className?:string}) {
  return <img aria-hidden="true" src={glyphs[name]} width={size} height={size} className={`svg-image ${className}`}/>;
}
// PI 标记用“圆底 + π 字形遮罩”合成：浅色下黑底白 π，深色下自动反过来。
// 两张固定配色的导出图里，白色变体在深色主题下会变成纯白圆块。
export function PiIcon({size=28}:{size?:number}) {
  return <span aria-hidden="true" className="pi-badge" style={{width:size,height:size,'--pi-glyph':`url("${piGlyph}")`} as CSSProperties}><span className="pi-badge-glyph"/></span>;
}
// 应用强调色：图标统一用蓝 / 白 / 黑，深浅主题各一套值。
export const ACCENT='#2E6FE4';
export const ACCENT_DARK='#7FB0FF';
export function AccentIcon({name,size=16}:{name:GlyphName;size?:number}) {
  return <TintIcon name={name} size={size} light={ACCENT} dark={ACCENT_DARK}/>;
}
export function SettingsIcon({width=20,height=20}:{width?:number;height?:number}) {
  return <AssetIcon name="gearshape2" size={Math.max(width,height)} />;
}
