import QtQuick
import QtQuick.Dialogs
import Quickshell
import qs.Ui
import qs.Commons

BarWidget {
  id: root
  moduleName: "tcballard.sortinghat"

  readonly property var sortingService: bar?.shell?.serviceFor("tcballard.sortinghat")
  readonly property int reviewCount: sortingService ? sortingService.awaitingReview : 0
  readonly property bool runtimeHealthy: sortingService
    && (sortingService.runtimeState === "running" || sortingService.runtimeState === "paused")
  property bool popupOpen: false
  property var folderProposal: null

  function close() { popupOpen = false }

  implicitWidth: badge.implicitWidth + Style.space(14)
  implicitHeight: barSize
  activeFocusOnTab: true
  Keys.onPressed: function(event) {
    if (event.key === Qt.Key_Return || event.key === Qt.Key_Space) {
      root.popupOpen = !root.popupOpen
      event.accepted = true
    } else if (event.key === Qt.Key_Escape) {
      root.popupOpen = false
      event.accepted = true
    }
  }

  Row {
    id: badge
    anchors.centerIn: parent
    spacing: Style.space(5)

    Text {
      textFormat: Text.PlainText
      text: root.runtimeHealthy ? "󰱼" : "󰀦"
      color: root.runtimeHealthy ? root.bar.barForeground : Color.warning
      font.family: root.bar.fontFamily
      font.pixelSize: Style.font.body
    }
    Text {
      textFormat: Text.PlainText
      text: String(root.reviewCount)
      color: root.bar.barForeground
      font.family: root.bar.fontFamily
      font.pixelSize: Style.font.body
      font.bold: root.reviewCount > 0
    }
  }

  MouseArea {
    anchors.fill: parent
    cursorShape: Qt.PointingHandCursor
    onClicked: root.popupOpen = !root.popupOpen
    onEntered: if (root.bar) root.bar.showTooltip(root, root.reviewCount + " files awaiting review")
    onExited: if (root.bar) root.bar.hideTooltip(root)
  }

  PopupCard {
    id: popup
    anchorItem: root
    bar: root.bar
    owner: root
    open: root.popupOpen
    contentWidth: popup.fittedContentWidth(Style.space(560))
    contentHeight: popup.fittedContentHeight(Math.min(Style.space(620), contentColumn.implicitHeight))

    Flickable {
      anchors.fill: parent
      contentHeight: contentColumn.implicitHeight
      clip: true

      Column {
        id: contentColumn
        width: parent.width
        spacing: Style.space(10)

        Row {
          width: parent.width
          spacing: Style.space(8)
          Text {
            textFormat: Text.PlainText
            text: "SortingHat review"
            color: root.bar.foreground
            font.family: root.bar.fontFamily
            font.pixelSize: Style.font.subtitle
            font.bold: true
          }
          Button {
            text: "Watch folder"
            foreground: root.bar.foreground
            focusable: true
            onClicked: watchedFolderDialog.open()
          }
          Button {
            text: root.sortingService && root.sortingService.runtimeState === "paused" ? "Resume" : "Pause"
            foreground: root.bar.foreground
            focusable: true
            enabled: root.runtimeHealthy
            onClicked: root.sortingService.run([
              root.sortingService.runtimeState === "paused" ? "resume" : "pause"
            ])
          }
        }

        Text {
          textFormat: Text.PlainText
          width: parent.width
          wrapMode: Text.Wrap
          visible: !root.runtimeHealthy || (root.sortingService && root.sortingService.lastError !== "")
          text: root.sortingService
            ? (root.sortingService.lastError || "Watcher is " + root.sortingService.runtimeState)
            : "SortingHat service is loading"
          color: Color.warning
          font.family: root.bar.fontFamily
          font.pixelSize: Style.font.bodySmall
        }

        Text {
          textFormat: Text.PlainText
          visible: root.runtimeHealthy && root.sortingService.queue.length === 0
          text: "Nothing is waiting for review."
          color: root.bar.foreground
          font.family: root.bar.fontFamily
          font.pixelSize: Style.font.body
        }

        Repeater {
          model: root.sortingService ? root.sortingService.queue : []

          BorderSurface {
            id: proposalCard
            required property var modelData
            width: contentColumn.width
            height: proposalContent.implicitHeight + Style.space(16)
            radius: Style.spacing.labelGap
            color: "transparent"
            borderSpec: Border.controlSpec("normal", root.bar.foreground, Color.accent)

            Column {
              id: proposalContent
              anchors.left: parent.left
              anchors.right: parent.right
              anchors.verticalCenter: parent.verticalCenter
              anchors.margins: Style.space(8)
              spacing: Style.space(5)

              Text {
                textFormat: Text.PlainText
                width: parent.width
                elide: Text.ElideMiddle
                text: "Source: " + String(proposalCard.modelData.source_display || "")
                color: root.bar.foreground
                font.family: root.bar.fontFamily
                font.pixelSize: Style.font.bodySmall
              }
              Text {
                textFormat: Text.PlainText
                width: parent.width
                elide: Text.ElideMiddle
                text: "Destination: " + String(proposalCard.modelData.destination_display || "Choose a folder")
                color: root.bar.foreground
                font.family: root.bar.fontFamily
                font.pixelSize: Style.font.bodySmall
              }
              Text {
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                text: "Why: " + String(proposalCard.modelData.reason || "")
                  + " · " + String(proposalCard.modelData.provenance || "unknown")
                color: root.bar.foreground
                font.family: root.bar.fontFamily
                font.pixelSize: Style.font.caption
              }
              Text {
                textFormat: Text.PlainText
                width: parent.width
                wrapMode: Text.Wrap
                visible: String(proposalCard.modelData.warning || "") !== ""
                text: String(proposalCard.modelData.warning || "")
                color: Color.warning
                font.family: root.bar.fontFamily
                font.pixelSize: Style.font.caption
              }

              Row {
                spacing: Style.space(5)
                Button {
                  text: "Move"
                  foreground: root.bar.foreground
            focusable: true
                  visible: proposalCard.modelData.status === "proposed"
                  enabled: proposalCard.modelData.destination !== null
                  onClicked: root.sortingService.run([
                    "proposal", "approve", proposalCard.modelData.id,
                    "--revision", proposalCard.modelData.revision
                  ])
                }
                Button {
                  text: "Choose folder"
                  foreground: root.bar.foreground
            focusable: true
                  visible: proposalCard.modelData.status === "proposed"
                  onClicked: {
                    root.folderProposal = proposalCard.modelData
                    destinationFolderDialog.open()
                  }
                }
                Button {
                  text: "Create rule"
                  foreground: root.bar.foreground
            focusable: true
                  visible: proposalCard.modelData.status === "proposed"
                  enabled: proposalCard.modelData.destination !== null
                  onClicked: root.sortingService.run([
                    "rule", "create", "--proposal", proposalCard.modelData.id
                  ])
                }
                Button {
                  text: "Ignore"
                  foreground: root.bar.foreground
            focusable: true
                  visible: proposalCard.modelData.status === "proposed"
                  onClicked: root.sortingService.run([
                    "proposal", "ignore", proposalCard.modelData.id,
                    "--revision", proposalCard.modelData.revision
                  ])
                }
                Button {
                  text: "Undo"
                  foreground: root.bar.foreground
            focusable: true
                  visible: proposalCard.modelData.status === "completed"
                  onClicked: root.sortingService.run([
                    "undo", proposalCard.modelData.id,
                    "--revision", proposalCard.modelData.revision
                  ])
                }
              }

              Row {
                spacing: Style.space(5)
                visible: proposalCard.modelData.status === "proposed"
                TextInput {
                  id: renameInput
                  width: Style.space(220)
                  text: String(proposalCard.modelData.destination_name || "")
                  color: root.bar.foreground
                  font.family: root.bar.fontFamily
                  font.pixelSize: Style.font.bodySmall
                  selectByMouse: true
                }
                Button {
                  text: "Rename"
                  foreground: root.bar.foreground
            focusable: true
                  enabled: proposalCard.modelData.destination !== null && renameInput.text !== ""
                  onClicked: root.sortingService.run([
                    "proposal", "rename", proposalCard.modelData.id, renameInput.text,
                    "--revision", proposalCard.modelData.revision
                  ])
                }
              }
            }
          }
        }
      }
    }
  }

  FolderDialog {
    id: watchedFolderDialog
    title: "Select a folder to watch"
    onAccepted: {
      var path = decodeURIComponent(String(selectedFolder).replace(/^file:\/\//, ""))
      if (root.sortingService) root.sortingService.addWatchedRoot(path)
    }
  }

  FolderDialog {
    id: destinationFolderDialog
    title: "Choose a destination folder"
    onAccepted: {
      if (!root.sortingService || !root.folderProposal) return
      var path = decodeURIComponent(String(selectedFolder).replace(/^file:\/\//, ""))
      root.sortingService.run([
        "proposal", "choose-folder", root.folderProposal.id, path,
        "--revision", root.folderProposal.revision
      ])
      root.folderProposal = null
    }
  }
}
