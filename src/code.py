from adafruit_macropad import MacroPad
import time

# Constants
KEY_FUNCS = ["Toggle Discord Mute", "", "Change Light Mode", "", "", "", "", "", "", "", "", ""]
NUM_MODES = 6
MESSAGE_REMOVE = 1000
TITLE = "behold!"
DEFAULT_BRIGHTNESS = 0.05

# Init 
macropad = MacroPad()
text_lines = macropad.display_text(title=TITLE)
macropad.pixels.brightness = DEFAULT_BRIGHTNESS

# State Variables
mode = 1
last_position = 0
till_message_remove = 0

while True:
    key_event = macropad.keys.events.get()
    
    till_message_remove = till_message_remove - 1 if till_message_remove > 0 else 0
    if till_message_remove == 0:
        text_lines[0].text=" "
        text_lines[1].text=" "

    if key_event:
        if key_event.pressed:
            till_message_remove = MESSAGE_REMOVE
            text_lines[0].text=KEY_FUNCS[key_event.key_number]
            macropad.pixels[key_event.key_number] = (52, 199, 220)

            # Discord Mute
            if key_event.key_number == 0:
                macropad.keyboard.press(
                    macropad.Keycode.CAPS_LOCK
                )
                macropad.keyboard.release_all()

            # Change Light Mode
            if key_event.key_number == 2:
                mode = mode+1 if mode < 4 else 1
                brightness = round(mode/NUM_MODES,2)
                macropad.pixels.brightness = brightness
                till_message_remove = MESSAGE_REMOVE
                text_lines[1].text="Light Mode: {}".format(brightness)
            # TODO RGB MODES and stuff

    macropad.encoder_switch_debounced.update()

    if macropad.encoder_switch_debounced.pressed:
        macropad.consumer_control.send(
                macropad.ConsumerControlCode.PLAY_PAUSE
        )

    current_position = macropad.encoder

    if macropad.encoder > last_position:
        macropad.consumer_control.send(
                macropad.ConsumerControlCode.VOLUME_INCREMENT
        )
        last_position = current_position

    if macropad.encoder < last_position:
        macropad.consumer_control.send(
                macropad.ConsumerControlCode.VOLUME_DECREMENT
        )
        last_position = current_position
    
    macropad.pixels.fill((219, 123, 48))

    text_lines.show()