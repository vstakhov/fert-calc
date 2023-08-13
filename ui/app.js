
// Sort elements returned from Rust by priority (if any), excluding insignificant elements
function sort_and_filter_elements(input) {
  return input
    .filter(function(e) { return !e.element.insignificant})
    .sort(function(a, b) {
      var elta = a.element;
      var eltb = b.element;
      if (elta.priority && eltb.priority) {
        return elta.priority < eltb.priority;
      }
      else if (elta.priority) {
        return -1;
      }
      else if (eltb.priority) {
        return 1;
      }
      else {
        return elta.name > eltb.name;
      }
    });
}

function html_from_concentrations(data) {
  var sorted_data = sort_and_filter_elements(data);
  var preamble = '<h2>Compound information</h2>';
  var header = '<table class="table table-striped table-hover"><thead><tr><th scope="col">Element/Alias</th><th scope="col">Concentration (%)</th></thead>';
  var rows = sorted_data
    .map(function(entry) {
      var name = entry.element.name;
      var res = '<tr><td>' + entry.element.name + '</td><td>' + (entry.concentration * 100.0).toFixed(3) + '</td></tr>';

      if (entry.aliases) {
        res += entry.aliases.map(alias => '<tr class="table-info"><td>' + alias.element_alias + '</td><td>' + (alias.concentration * 100.0).toFixed(3) + '</td></tr>').join('');
      }
      return res;
    }).join('');

    return preamble + header + rows + "</table>";
}

function html_from_calculation(data, isTarget, fertilizer) {
  var sorted_data = sort_and_filter_elements(data.elements_dose);
  var dosage = "<h2>Calculation results</h2>";

  if (isTarget) {
    dosage += "<div class=\"container\">You need to add <strong>" + data.compound_dose.toFixed(3) + "g</strong> of the fertilizer (<i>" + fertilizer +
     "</i>) to reach the following concentrations:</div>";
  }
  else {
    dosage += "<div class=\"container\">After adding <strong>" + data.compound_dose.toFixed(3) + "g</strong> of the fertilizer (<i>" + fertilizer + 
    "</i>) you will reach the following concentrations:</div>";
  }

  var header = '<table class="table table-striped table-hover"><thead><tr><th scope="col">Element/Alias</th><th scope="col">Concentration (ppm)</th></thead>';
  var rows = sorted_data
    .map(function(entry) {
      var name = entry.element.name;
      var res = '<tr><td>' + entry.element.name + '</td><td>' + (entry.dose).toFixed(3) + '</td></tr>';

      if (entry.aliases) {
        res += entry.aliases.map(alias => '<tr class="table-info"><td>' + alias.element_alias + '</td><td>' + (alias.dose).toFixed(3) + '</td></tr>').join('');
      }
      return res;
    }).join('');

    return dosage + header + rows + "</table>";
}

$(function() {
  // Setup radio buttons and forms
  var volumeDimensions = false;
  var compoundPremixed = true;
  var dosingSolution = false;
  var targetDose = false;


  $("#volumeRadio").click(function() {
    volumeDimensions = false;
    $("#formDimensions").hide();
    $("#formVolume").show();
  });
  $("#dimRadio").click(function() {
    volumeDimensions = true;
    $("#formVolume").hide();
    $("#formDimensions").show();
  });

  $("#premixRadio").click(function() {
    $("#formCompound").hide();
    $("#formPremixed").show();
    compoundPremixed = true;
  });
  $("#compoundRadio").click(function() {
    $("#formPremixed").hide();
    $("#formCompound").show();
    compoundPremixed = false;
  });

  $("#dryRadio").click(function() {
    $("#formSolutionDosing").hide();
    dosingSolution = false;
  });
  $("#solutionRadio").click(function() {
    $("#formSolutionDosing").show();
    dosingSolution = true;
  });

  $("#resultDoseRadio").click(function() {
    targetDose = false;
    $("#formResultDose").show();
    $("#formTargetDose").hide();
  });
  $("#targetDoseRadio").click(function() {
    $("#formResultDose").hide();
    $("#formTargetDose").show();
    targetDose = true;
  });

  $("#mixSelect").change(function() {
    if (this.value == "") {
      return;
    }
    localStorage.setItem("selected_mix", this.value);
    $.getJSON("/info/" + this.value, function(data) {
      $("#resultTable").html(html_from_concentrations(data));
    });
  })
  .change();

  $('#compoundInput').bind("enterKey",function(e){
    if (this.value == "") {
      return;
    }
    $.getJSON("/info/" + this.value, function(data) {
      $("#resultTable").html(html_from_concentrations(data));
    });
  });
  $('#compoundInput').keyup(function(e){
     if(e.keyCode == 13) {
        $(this).trigger("enterKey");
     }
  });
  $('#compoundInput').focusout(function(){
    $(this).trigger("enterKey");
  });
 
  
  // Hide non-default forms
  $("#formDimensions").hide();
  $("#formCompound").hide();
  $("#formSolutionDosing").hide();
  $("#formTargetDose").hide();

  // Enable language panel
  $('.switch-btn').click(function() {
    var lang = $(this).data('lang');
    $('h2').hide();
    $('h2.' + lang).show();
    $('.btn-outline-secondary').hide();
    $('.btn-outline-secondary.' + lang).show();
    $('.form-label').hide();
    $('.form-label.' + lang).show();
    $('.form-check-label').hide();
    $('.form-check-label.' + lang).show();
  });

  // Load fertilizers
  $.getJSON("/list", function(data) {
    $.each(data, function(idx, val) {
      $('#mixSelect').append($('<option>', { 
        value: val[0],
        text : val[0],
        "data-bs-toggle": "tooltip", 
        title: val[1]
      }));
    });

    $('#mixSelect').attr("size", Math.min(data.length, 32));

    var prev_selection = localStorage.getItem("selected_mix");
    if (prev_selection) {
      $('#mixSelect').val(prev_selection);
      $('#mixSelect').trigger("change");
    }
  });
  // Submit button logic
  $('#submitButton').click(function() {
    var request = {}

    var required_checks = {}

    if (compoundPremixed) {
      request.fertilizer = $('#mixSelect').val();
      required_checks.mixSelect = "string";
    }
    else {
      request.fertilizer = $('#compoundInput').val();
      required_checks.compoundInput = "string";
    }

    if (volumeDimensions) {
      request.tank = {
        "volume": {
          "height": parseFloat($('#tankHeight').val()) / 10.0,
          "width": parseFloat($('#tankWidth').val()) / 10.0,
          "length": parseFloat($('#tankLength').val()) / 10.0,
        }
      }
      required_checks.tankHeight = "number";
      required_checks.tankWidth = "number";
      required_checks.tankLength = "number";
    }
    else {
      request.tank = {
        "volume": parseFloat($('#tankVolume').val()),
        "absolute": $('#checkAbsoluteVolume').is(':checked')
      }
      required_checks.tankVolume = "number";
    }

    var dosing_data = {}
    if (dosingSolution) {
      dosing_data.portion_volume = parseFloat($('#doseVolume').val());
      dosing_data.container_volume = parseFloat($('#solutionVolume').val());
      dosing_data.type = "Solution";

      required_checks.doseVolume = "number";
      required_checks.solutionVolume = "number";
    }
    else {
      dosing_data.type = "Dry";
    }
  
    if (targetDose) {
      required_checks.targetConcentration = "number";
      required_checks.targetCompound = "string";
      if (dosingSolution) {
        dosing_data.solution_input = parseFloat($('#targetConcentration').val());
      }
      else {
        dosing_data.dilute_input = parseFloat($('#targetConcentration').val());
      }
      dosing_data.target_element = $('#targetCompound').val();
      dosing_data.what = "TargetDose";
    }
    else {
      required_checks.addedWeight = "number";
      if (dosingSolution) {
        dosing_data.solution_input = parseFloat($('#addedWeight').val());
      }
      else {
        dosing_data.dilute_input = parseFloat($('#addedWeight').val());
      }
      dosing_data.what = "ResultOfDose";
    }

    request.dosing_data = dosing_data;

    var check_failed = false;
    for (const [k, v] of Object.entries(required_checks)) {
      let val = $("#" + k).val();

      if (!val) {
        alert("Missing required input: " + k);
        $("#" + k).focus();
        check_failed = true;
        break;
      }
      else {
        if (v == "number") {
          let num = parseFloat(val);

          if (isNaN(num)) {
            alert("Invalid numeric value for field" + k + ": " + val);
            $("#" + k).focus();
            check_failed = true;
            break;
          }
        }
      }
    }

    $.ajax({
      type: "POST",
      url: "/calc",
      contentType: "application/json",
      data: JSON.stringify(request),
      success: function(data) {
        $("#resultTable").html(html_from_calculation(data, targetDose, request.fertilizer));
      },
      dataType: "json"
    });
  });
});
