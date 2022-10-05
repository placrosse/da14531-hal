use crate::pac::GPADC;

pub mod config;

use config::AdcConfig;

/// Extension trait that constrains the `SYS_WDOG` peripheral
pub trait GpAdcExt {
    /// Constrains the `SYS_WDOG` peripheral so it plays nicely with the other abstractions
    fn constrain(self) -> GpAdc;
}

impl GpAdcExt for GPADC {
    fn constrain(self) -> GpAdc {
        GpAdc { gpadc: self }
    }
}

pub struct GpAdc {
    gpadc: GPADC,
}

impl GpAdc {
    pub fn init(&self, adc_config: AdcConfig) {
        self.reset();

        // Set GP_ADC_LDO_LEVEL to the preferred level of 925mV
        self.gpadc
            .gp_adc_trim_reg
            .modify(|_, w| unsafe { w.gp_adc_ldo_level().bits(0x4) });

        self.enable();
        self.configure(adc_config);
    }

    pub fn configure(&self, adc_config: AdcConfig) {
        self.gpadc.gp_adc_sel_reg.modify(|_, w| unsafe {
            w.gp_adc_sel_p().bits(adc_config.channel_sel_pos);
            w.gp_adc_sel_n().bits(adc_config.channel_sel_neg)
        });

        self.gpadc.gp_adc_ctrl_reg.modify(|_, w| {
            w.gp_adc_se().bit(adc_config.mode.into());
            w.gp_adc_cont().bit(adc_config.continuous.into());
            w.die_temp_en().bit(adc_config.enable_die_temp);
            w.gp_adc_chop().bit(adc_config.chopper.into())
        });

        if adc_config.enable_die_temp {
            // Guideline from the Analog IC Team: Wait for 25us to let the temperature
            // sensor settle just after enabling it
            // 25us*16MHz = 400 cylces
            crate::cm::asm::delay(400);
        }

        self.gpadc.gp_adc_ctrl2_reg.modify(|_, w| unsafe {
            w.gp_adc_attn().bits(adc_config.attenuation as u8);
            w.gp_adc_conv_nrs().bits(adc_config.averaging as u8);
            w.gp_adc_smpl_time().bits(adc_config.sample_time as u8)
        });
    }

    /// Enable ADC peripheral
    pub fn enable(&self) {
        self.gpadc
            .gp_adc_ctrl_reg
            .modify(|_, w| w.gp_adc_en().set_bit());

        let delay_cycles = 4 * self.gpadc.gp_adc_ctrl3_reg.read().gp_adc_en_del().bits() as u32;

        crate::cm::asm::delay(delay_cycles);
    }

    /// Disable ADC peripheral
    pub fn disable(&self) {
        self.gpadc
            .gp_adc_ctrl_reg
            .modify(|_, w| w.gp_adc_en().clear_bit());
    }

    /// Reset ADC peripheral
    pub fn reset(&self) {
        self.gpadc.gp_adc_ctrl_reg.reset();
        self.gpadc.gp_adc_ctrl2_reg.reset();
        self.gpadc.gp_adc_ctrl3_reg.reset();
        self.gpadc.gp_adc_sel_reg.reset();
        self.gpadc.gp_adc_trim_reg.reset();
    }

    /// Start ADC conversion
    pub fn start_conversion(&self) {
        self.gpadc
            .gp_adc_ctrl_reg
            .modify(|_, w| w.gp_adc_start().set_bit());
    }

    /// Wait for conversion to finish (in manual mode only)
    pub fn wait_for_conversion(&self) {
        while self
            .gpadc
            .gp_adc_ctrl_reg
            .read()
            .gp_adc_start()
            .bit_is_set()
        {}

        self.gpadc
            .gp_adc_clear_int_reg
            .write(|w| unsafe { w.gp_adc_clr_int().bits(1) })
    }

    /// Read current sample value from register
    pub fn current_sample(&self) -> u16 {
        self.gpadc.gp_adc_result_reg.read().gp_adc_val().bits()
    }
}
